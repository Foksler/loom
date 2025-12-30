// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

use async_trait::async_trait;
use futures::StreamExt;
use k8s_openapi::api::core::v1::{Namespace, Pod};
use kube::{
	api::{Api, DeleteParams, ListParams, LogParams, PostParams},
	Client,
};
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tracing::debug;

use crate::client::K8sClient;
use crate::error::K8sError;
use crate::types::{LogOptions, LogStream};

/// Production K8s client implementation using the kube crate.
pub struct KubeClient {
	client: Client,
}

impl KubeClient {
	/// Create a new KubeClient that auto-discovers cluster configuration.
	///
	/// This will attempt to load config from:
	/// 1. In-cluster service account (when running in K8s)
	/// 2. KUBECONFIG environment variable
	/// 3. ~/.kube/config
	pub async fn new() -> Result<Self, K8sError> {
		let client = Client::try_default().await?;
		debug!("K8s client initialized");
		Ok(Self { client })
	}
}

#[async_trait]
impl K8sClient for KubeClient {
	async fn create_pod(&self, namespace: &str, pod: Pod) -> Result<Pod, K8sError> {
		let pods: Api<Pod> = Api::namespaced(self.client.clone(), namespace);
		let pod = pods.create(&PostParams::default(), &pod).await?;
		Ok(pod)
	}

	async fn delete_pod(
		&self,
		name: &str,
		namespace: &str,
		grace_period_seconds: u32,
	) -> Result<(), K8sError> {
		let pods: Api<Pod> = Api::namespaced(self.client.clone(), namespace);
		let dp = DeleteParams {
			grace_period_seconds: Some(grace_period_seconds),
			..Default::default()
		};
		match pods.delete(name, &dp).await {
			Ok(_) => Ok(()),
			Err(kube::Error::Api(err)) if err.code == 404 => {
				Err(K8sError::PodNotFound { name: name.into() })
			}
			Err(e) => Err(e.into()),
		}
	}

	async fn list_pods(&self, namespace: &str, label_selector: &str) -> Result<Vec<Pod>, K8sError> {
		let pods: Api<Pod> = Api::namespaced(self.client.clone(), namespace);
		let lp = ListParams::default().labels(label_selector);
		let pod_list = pods.list(&lp).await?;
		Ok(pod_list.items)
	}

	async fn get_pod(&self, name: &str, namespace: &str) -> Result<Pod, K8sError> {
		let pods: Api<Pod> = Api::namespaced(self.client.clone(), namespace);
		match pods.get(name).await {
			Ok(pod) => Ok(pod),
			Err(kube::Error::Api(err)) if err.code == 404 => {
				Err(K8sError::PodNotFound { name: name.into() })
			}
			Err(e) => Err(e.into()),
		}
	}

	async fn get_namespace(&self, name: &str) -> Result<Namespace, K8sError> {
		let namespaces: Api<Namespace> = Api::all(self.client.clone());
		match namespaces.get(name).await {
			Ok(ns) => Ok(ns),
			Err(kube::Error::Api(err)) if err.code == 404 => {
				Err(K8sError::NamespaceNotFound { name: name.into() })
			}
			Err(e) => Err(e.into()),
		}
	}

	async fn stream_logs(
		&self,
		name: &str,
		namespace: &str,
		container: &str,
		opts: LogOptions,
	) -> Result<LogStream, K8sError> {
		let pods: Api<Pod> = Api::namespaced(self.client.clone(), namespace);
		let lp = LogParams {
			container: Some(container.to_string()),
			follow: true,
			tail_lines: Some(opts.tail.into()),
			timestamps: opts.timestamps,
			..Default::default()
		};

		let stream = pods.log_stream(name, &lp).await.map_err(|e| match e {
			kube::Error::Api(ref err) if err.code == 404 => K8sError::PodNotFound { name: name.into() },
			_ => K8sError::StreamError {
				message: e.to_string(),
			},
		})?;

		let compat_stream = stream.compat();
		let lines_stream = tokio_util::io::ReaderStream::new(compat_stream);
		let mapped = lines_stream.map(|result| {
			result.map_err(std::io::Error::other)
		});
		Ok(Box::pin(mapped))
	}
}
