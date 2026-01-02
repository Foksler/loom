// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

#![cfg(feature = "sink-json-stream")]

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::Mutex;

use crate::enrichment::EnrichedAuditEvent;
use crate::error::AuditSinkError;
use crate::filter::AuditFilterConfig;
use crate::sink::AuditSink;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamProtocol {
	Tcp,
	Udp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonStreamConfig {
	pub name: String,
	pub host: String,
	pub port: u16,
	pub protocol: StreamProtocol,
	pub filter: AuditFilterConfig,
}

pub struct JsonStreamSink {
	config: JsonStreamConfig,
	tcp_stream: Mutex<Option<TcpStream>>,
	udp_socket: Mutex<Option<UdpSocket>>,
}

impl JsonStreamSink {
	pub fn new(config: JsonStreamConfig) -> Self {
		Self {
			config,
			tcp_stream: Mutex::new(None),
			udp_socket: Mutex::new(None),
		}
	}

	fn address(&self) -> String {
		format!("{}:{}", self.config.host, self.config.port)
	}

	async fn connect_tcp(&self) -> Result<TcpStream, AuditSinkError> {
		TcpStream::connect(self.address())
			.await
			.map_err(|e| AuditSinkError::Transient(format!("TCP connect failed: {e}")))
	}

	async fn get_or_connect_tcp(&self) -> Result<(), AuditSinkError> {
		let mut guard = self.tcp_stream.lock().await;
		if guard.is_none() {
			*guard = Some(self.connect_tcp().await?);
		}
		Ok(())
	}

	async fn send_tcp(&self, data: &[u8]) -> Result<(), AuditSinkError> {
		let mut guard = self.tcp_stream.lock().await;

		if guard.is_none() {
			*guard = Some(self.connect_tcp().await?);
		}

		let stream = guard.as_mut().unwrap();
		match stream.write_all(data).await {
			Ok(()) => Ok(()),
			Err(e) => {
				*guard = None;
				Err(AuditSinkError::Transient(format!("TCP write failed: {e}")))
			}
		}
	}

	async fn send_udp(&self, data: &[u8]) -> Result<(), AuditSinkError> {
		let mut guard = self.udp_socket.lock().await;

		if guard.is_none() {
			let socket = UdpSocket::bind("0.0.0.0:0")
				.await
				.map_err(|e| AuditSinkError::Transient(format!("UDP bind failed: {e}")))?;
			socket
				.connect(self.address())
				.await
				.map_err(|e| AuditSinkError::Transient(format!("UDP connect failed: {e}")))?;
			*guard = Some(socket);
		}

		let socket = guard.as_ref().unwrap();
		socket
			.send(data)
			.await
			.map_err(|e| AuditSinkError::Transient(format!("UDP send failed: {e}")))?;
		Ok(())
	}
}

#[async_trait]
impl AuditSink for JsonStreamSink {
	fn name(&self) -> &str {
		&self.config.name
	}

	fn filter(&self) -> &AuditFilterConfig {
		&self.config.filter
	}

	async fn publish(&self, event: Arc<EnrichedAuditEvent>) -> Result<(), AuditSinkError> {
		let mut json = serde_json::to_string(event.as_ref())
			.map_err(|e| AuditSinkError::Permanent(format!("JSON serialization failed: {e}")))?;
		json.push('\n');
		let data = json.as_bytes();

		match self.config.protocol {
			StreamProtocol::Tcp => self.send_tcp(data).await,
			StreamProtocol::Udp => self.send_udp(data).await,
		}
	}

	async fn health_check(&self) -> Result<(), AuditSinkError> {
		match self.config.protocol {
			StreamProtocol::Tcp => {
				self.get_or_connect_tcp().await?;
				Ok(())
			}
			StreamProtocol::Udp => {
				let socket = UdpSocket::bind("0.0.0.0:0")
					.await
					.map_err(|e| AuditSinkError::Transient(format!("UDP bind failed: {e}")))?;
				socket
					.connect(self.address())
					.await
					.map_err(|e| AuditSinkError::Transient(format!("UDP connect failed: {e}")))?;
				socket
					.send(b"")
					.await
					.map_err(|e| AuditSinkError::Transient(format!("UDP ping failed: {e}")))?;
				Ok(())
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::event::{AuditEventType, AuditLogEntry, AuditSeverity};

	fn make_config(protocol: StreamProtocol) -> JsonStreamConfig {
		JsonStreamConfig {
			name: "test-stream".to_string(),
			host: "127.0.0.1".to_string(),
			port: 9999,
			protocol,
			filter: AuditFilterConfig::default(),
		}
	}

	fn make_event() -> EnrichedAuditEvent {
		EnrichedAuditEvent {
			base: AuditLogEntry::builder(AuditEventType::Login)
				.severity(AuditSeverity::Info)
				.build(),
			session: None,
			org: None,
		}
	}

	#[test]
	fn test_stream_protocol_serde() {
		let tcp_json = serde_json::to_string(&StreamProtocol::Tcp).unwrap();
		assert_eq!(tcp_json, "\"tcp\"");

		let udp_json = serde_json::to_string(&StreamProtocol::Udp).unwrap();
		assert_eq!(udp_json, "\"udp\"");

		let tcp: StreamProtocol = serde_json::from_str("\"tcp\"").unwrap();
		assert_eq!(tcp, StreamProtocol::Tcp);

		let udp: StreamProtocol = serde_json::from_str("\"udp\"").unwrap();
		assert_eq!(udp, StreamProtocol::Udp);
	}

	#[test]
	fn test_json_stream_config_serde() {
		let config = make_config(StreamProtocol::Tcp);
		let json = serde_json::to_string(&config).unwrap();
		let parsed: JsonStreamConfig = serde_json::from_str(&json).unwrap();

		assert_eq!(parsed.name, config.name);
		assert_eq!(parsed.host, config.host);
		assert_eq!(parsed.port, config.port);
		assert_eq!(parsed.protocol, config.protocol);
	}

	#[test]
	fn test_sink_name_and_filter() {
		let config = make_config(StreamProtocol::Udp);
		let sink = JsonStreamSink::new(config.clone());

		assert_eq!(sink.name(), "test-stream");
		assert_eq!(sink.filter().min_severity, AuditSeverity::Info);
	}

	#[test]
	fn test_address_formatting() {
		let config = JsonStreamConfig {
			name: "test".to_string(),
			host: "logstash.example.com".to_string(),
			port: 5044,
			protocol: StreamProtocol::Tcp,
			filter: AuditFilterConfig::default(),
		};
		let sink = JsonStreamSink::new(config);
		assert_eq!(sink.address(), "logstash.example.com:5044");
	}

	#[tokio::test]
	async fn test_event_serialization_has_newline() {
		let event = make_event();
		let mut json = serde_json::to_string(&event).unwrap();
		json.push('\n');

		assert!(json.ends_with('\n'));
		assert!(json.contains("\"event_type\""));
		assert!(json.contains("\"severity\""));
	}

	#[tokio::test]
	async fn test_tcp_connect_fails_gracefully() {
		let config = make_config(StreamProtocol::Tcp);
		let sink = JsonStreamSink::new(config);
		let event = Arc::new(make_event());

		let result = sink.publish(event).await;
		assert!(result.is_err());

		if let Err(AuditSinkError::Transient(msg)) = result {
			assert!(msg.contains("TCP"));
		} else {
			panic!("Expected transient TCP error");
		}
	}

	#[tokio::test]
	async fn test_health_check_tcp_fails_gracefully() {
		let config = make_config(StreamProtocol::Tcp);
		let sink = JsonStreamSink::new(config);

		let result = sink.health_check().await;
		assert!(result.is_err());
	}
}
