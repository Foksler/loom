// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use tracing::instrument;

use loom_analytics_core::{
	AliasPayload, IdentifyPayload, MergeReason, OrgId, Person, PersonIdentity, PersonMerge,
	PersonWithIdentities,
};

use crate::error::{AnalyticsServerError, Result};
use crate::repository::AnalyticsRepository;

pub struct IdentityResolutionService<R: AnalyticsRepository> {
	repository: R,
}

impl<R: AnalyticsRepository> IdentityResolutionService<R> {
	pub fn new(repository: R) -> Self {
		Self { repository }
	}

	#[instrument(skip(self), fields(org_id = %org_id, distinct_id = %distinct_id))]
	pub async fn resolve_person_for_distinct_id(
		&self,
		org_id: OrgId,
		distinct_id: &str,
	) -> Result<PersonWithIdentities> {
		if let Some(identity) = self
			.repository
			.get_identity_by_distinct_id(org_id, distinct_id)
			.await?
		{
			if let Some(person_with_identities) = self
				.repository
				.get_person_with_identities(identity.person_id)
				.await?
			{
				if person_with_identities.person.is_merged() {
					if let Some(merged_into_id) = person_with_identities.person.merged_into_id {
						if let Some(winner) = self
							.repository
							.get_person_with_identities(merged_into_id)
							.await?
						{
							return Ok(winner);
						}
					}
				}
				return Ok(person_with_identities);
			}
		}

		let person = Person::new(org_id);
		self.repository.create_person(&person).await?;

		let identity = PersonIdentity::anonymous(person.id, distinct_id.to_string());
		self.repository.create_identity(&identity).await?;

		Ok(PersonWithIdentities::new(person, vec![identity]))
	}

	#[instrument(skip(self, payload), fields(org_id = %org_id, distinct_id = %payload.distinct_id, user_id = %payload.user_id))]
	pub async fn identify(
		&self,
		org_id: OrgId,
		payload: IdentifyPayload,
	) -> Result<PersonWithIdentities> {
		let current_identity = self
			.repository
			.get_identity_by_distinct_id(org_id, &payload.distinct_id)
			.await?;
		let user_identity = self
			.repository
			.get_identity_by_distinct_id(org_id, &payload.user_id)
			.await?;

		match (current_identity, user_identity) {
			(None, None) => {
				let person = Person::new(org_id).with_properties(payload.properties.clone());
				self.repository.create_person(&person).await?;

				let anon_identity = PersonIdentity::anonymous(person.id, payload.distinct_id.clone());
				self.repository.create_identity(&anon_identity).await?;

				let identified_identity = PersonIdentity::identified(person.id, payload.user_id.clone());
				self
					.repository
					.create_identity(&identified_identity)
					.await?;

				Ok(PersonWithIdentities::new(
					person,
					vec![anon_identity, identified_identity],
				))
			}

			(Some(current), None) => {
				let mut person = self
					.repository
					.get_person_by_id(current.person_id)
					.await?
					.ok_or_else(|| {
						AnalyticsServerError::Internal("Person not found for identity".to_string())
					})?;

				person.set_properties(payload.properties.clone());
				self.repository.update_person(&person).await?;

				let identified_identity = PersonIdentity::identified(person.id, payload.user_id.clone());
				self
					.repository
					.create_identity(&identified_identity)
					.await?;

				let identities = self
					.repository
					.list_identities_for_person(person.id)
					.await?;
				Ok(PersonWithIdentities::new(person, identities))
			}

			(None, Some(user)) => {
				let mut person = self
					.repository
					.get_person_by_id(user.person_id)
					.await?
					.ok_or_else(|| {
						AnalyticsServerError::Internal("Person not found for identity".to_string())
					})?;

				person.set_properties(payload.properties.clone());
				self.repository.update_person(&person).await?;

				let anon_identity = PersonIdentity::anonymous(person.id, payload.distinct_id.clone());
				self.repository.create_identity(&anon_identity).await?;

				let identities = self
					.repository
					.list_identities_for_person(person.id)
					.await?;
				Ok(PersonWithIdentities::new(person, identities))
			}

			(Some(current), Some(user)) => {
				if current.person_id == user.person_id {
					let mut person = self
						.repository
						.get_person_by_id(current.person_id)
						.await?
						.ok_or_else(|| {
							AnalyticsServerError::Internal("Person not found for identity".to_string())
						})?;

					person.set_properties(payload.properties.clone());
					self.repository.update_person(&person).await?;

					let identities = self
						.repository
						.list_identities_for_person(person.id)
						.await?;
					return Ok(PersonWithIdentities::new(person, identities));
				}

				let current_person = self
					.repository
					.get_person_with_identities(current.person_id)
					.await?
					.ok_or_else(|| {
						AnalyticsServerError::Internal("Person not found for identity".to_string())
					})?;
				let user_person = self
					.repository
					.get_person_with_identities(user.person_id)
					.await?
					.ok_or_else(|| {
						AnalyticsServerError::Internal("Person not found for identity".to_string())
					})?;

				let (winner, loser) = self.determine_merge_winner(&current_person, &user_person);

				self
					.merge_persons(
						winner.person.id,
						loser.person.id,
						MergeReason::identify(payload.distinct_id.clone(), payload.user_id.clone()),
						Some(payload.properties.clone()),
					)
					.await
			}
		}
	}

	#[instrument(skip(self, payload), fields(org_id = %org_id, distinct_id = %payload.distinct_id, alias = %payload.alias))]
	pub async fn alias(&self, org_id: OrgId, payload: AliasPayload) -> Result<PersonWithIdentities> {
		let primary_identity = self
			.repository
			.get_identity_by_distinct_id(org_id, &payload.distinct_id)
			.await?;
		let alias_identity = self
			.repository
			.get_identity_by_distinct_id(org_id, &payload.alias)
			.await?;

		match (primary_identity, alias_identity) {
			(None, None) => {
				let person = Person::new(org_id);
				self.repository.create_person(&person).await?;

				let primary = PersonIdentity::anonymous(person.id, payload.distinct_id.clone());
				self.repository.create_identity(&primary).await?;

				let alias = PersonIdentity::anonymous(person.id, payload.alias.clone());
				self.repository.create_identity(&alias).await?;

				Ok(PersonWithIdentities::new(person, vec![primary, alias]))
			}

			(Some(primary), None) => {
				let person = self
					.repository
					.get_person_by_id(primary.person_id)
					.await?
					.ok_or_else(|| {
						AnalyticsServerError::Internal("Person not found for identity".to_string())
					})?;

				let alias = PersonIdentity::anonymous(person.id, payload.alias.clone());
				self.repository.create_identity(&alias).await?;

				let identities = self
					.repository
					.list_identities_for_person(person.id)
					.await?;
				Ok(PersonWithIdentities::new(person, identities))
			}

			(None, Some(alias)) => {
				let person = self
					.repository
					.get_person_by_id(alias.person_id)
					.await?
					.ok_or_else(|| {
						AnalyticsServerError::Internal("Person not found for identity".to_string())
					})?;

				let primary = PersonIdentity::anonymous(person.id, payload.distinct_id.clone());
				self.repository.create_identity(&primary).await?;

				let identities = self
					.repository
					.list_identities_for_person(person.id)
					.await?;
				Ok(PersonWithIdentities::new(person, identities))
			}

			(Some(primary), Some(alias)) => {
				if primary.person_id == alias.person_id {
					let person = self
						.repository
						.get_person_by_id(primary.person_id)
						.await?
						.ok_or_else(|| {
							AnalyticsServerError::Internal("Person not found for identity".to_string())
						})?;

					let identities = self
						.repository
						.list_identities_for_person(person.id)
						.await?;
					return Ok(PersonWithIdentities::new(person, identities));
				}

				let primary_person = self
					.repository
					.get_person_with_identities(primary.person_id)
					.await?
					.ok_or_else(|| {
						AnalyticsServerError::Internal("Person not found for identity".to_string())
					})?;
				let alias_person = self
					.repository
					.get_person_with_identities(alias.person_id)
					.await?
					.ok_or_else(|| {
						AnalyticsServerError::Internal("Person not found for identity".to_string())
					})?;

				let (winner, loser) = self.determine_merge_winner(&primary_person, &alias_person);

				self
					.merge_persons(
						winner.person.id,
						loser.person.id,
						MergeReason::alias(payload.distinct_id.clone(), payload.alias.clone()),
						None,
					)
					.await
			}
		}
	}

	fn determine_merge_winner<'a>(
		&self,
		person_a: &'a PersonWithIdentities,
		person_b: &'a PersonWithIdentities,
	) -> (&'a PersonWithIdentities, &'a PersonWithIdentities) {
		let a_identified = person_a.has_identified_identity();
		let b_identified = person_b.has_identified_identity();

		match (a_identified, b_identified) {
			(true, false) => (person_a, person_b),
			(false, true) => (person_b, person_a),
			_ => {
				if person_a.person.created_at <= person_b.person.created_at {
					(person_a, person_b)
				} else {
					(person_b, person_a)
				}
			}
		}
	}

	#[instrument(skip(self, additional_properties), fields(winner_id = %winner_id, loser_id = %loser_id))]
	async fn merge_persons(
		&self,
		winner_id: loom_analytics_core::PersonId,
		loser_id: loom_analytics_core::PersonId,
		reason: MergeReason,
		additional_properties: Option<serde_json::Value>,
	) -> Result<PersonWithIdentities> {
		let mut winner = self
			.repository
			.get_person_by_id(winner_id)
			.await?
			.ok_or_else(|| AnalyticsServerError::Internal("Winner person not found".to_string()))?;

		let loser = self
			.repository
			.get_person_by_id(loser_id)
			.await?
			.ok_or_else(|| AnalyticsServerError::Internal("Loser person not found".to_string()))?;

		if let (serde_json::Value::Object(winner_props), serde_json::Value::Object(loser_props)) =
			(&mut winner.properties, &loser.properties)
		{
			for (key, value) in loser_props {
				winner_props.entry(key.clone()).or_insert(value.clone());
			}
		}

		if let Some(props) = additional_properties {
			winner.set_properties(props);
		}

		self.repository.update_person(&winner).await?;

		self.repository.reassign_events(loser_id, winner_id).await?;

		self
			.repository
			.transfer_identities(loser_id, winner_id)
			.await?;

		let mut loser = loser;
		loser.merge_into(winner_id);
		self.repository.update_person(&loser).await?;

		let merge = PersonMerge::new(winner_id, loser_id, reason);
		self.repository.create_merge(&merge).await?;

		let identities = self
			.repository
			.list_identities_for_person(winner_id)
			.await?;
		Ok(PersonWithIdentities::new(winner, identities))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::repository::AnalyticsRepository;
	use async_trait::async_trait;
	use chrono::{DateTime, Utc};
	use loom_analytics_core::{
		AnalyticsApiKey, AnalyticsApiKeyId, Event, EventId, PersonId, PersonIdentityId,
	};
	use std::collections::HashMap;
	use std::sync::{Arc, Mutex};

	#[derive(Clone, Default)]
	struct MockRepository {
		persons: Arc<Mutex<HashMap<PersonId, Person>>>,
		identities: Arc<Mutex<HashMap<PersonIdentityId, PersonIdentity>>>,
		events: Arc<Mutex<HashMap<EventId, Event>>>,
		merges: Arc<Mutex<Vec<PersonMerge>>>,
	}

	impl MockRepository {
		fn new() -> Self {
			Self::default()
		}
	}

	#[async_trait]
	impl AnalyticsRepository for MockRepository {
		async fn create_person(&self, person: &Person) -> Result<()> {
			self
				.persons
				.lock()
				.unwrap()
				.insert(person.id, person.clone());
			Ok(())
		}

		async fn get_person_by_id(&self, id: PersonId) -> Result<Option<Person>> {
			Ok(self.persons.lock().unwrap().get(&id).cloned())
		}

		async fn get_person_with_identities(
			&self,
			id: PersonId,
		) -> Result<Option<PersonWithIdentities>> {
			let persons = self.persons.lock().unwrap();
			let identities = self.identities.lock().unwrap();

			let person = match persons.get(&id) {
				Some(p) => p.clone(),
				None => return Ok(None),
			};

			let person_identities: Vec<PersonIdentity> = identities
				.values()
				.filter(|i| i.person_id == id)
				.cloned()
				.collect();

			Ok(Some(PersonWithIdentities::new(person, person_identities)))
		}

		async fn list_persons(&self, org_id: OrgId, limit: u32, offset: u32) -> Result<Vec<Person>> {
			let persons = self.persons.lock().unwrap();
			let result: Vec<Person> = persons
				.values()
				.filter(|p| p.org_id == org_id && p.merged_into_id.is_none())
				.skip(offset as usize)
				.take(limit as usize)
				.cloned()
				.collect();
			Ok(result)
		}

		async fn update_person(&self, person: &Person) -> Result<()> {
			self
				.persons
				.lock()
				.unwrap()
				.insert(person.id, person.clone());
			Ok(())
		}

		async fn count_persons(&self, org_id: OrgId) -> Result<u64> {
			let persons = self.persons.lock().unwrap();
			let count = persons
				.values()
				.filter(|p| p.org_id == org_id && p.merged_into_id.is_none())
				.count();
			Ok(count as u64)
		}

		async fn create_identity(&self, identity: &PersonIdentity) -> Result<()> {
			self
				.identities
				.lock()
				.unwrap()
				.insert(identity.id, identity.clone());
			Ok(())
		}

		async fn get_identity_by_distinct_id(
			&self,
			org_id: OrgId,
			distinct_id: &str,
		) -> Result<Option<PersonIdentity>> {
			let identities = self.identities.lock().unwrap();
			let persons = self.persons.lock().unwrap();

			for identity in identities.values() {
				if identity.distinct_id == distinct_id {
					if let Some(person) = persons.get(&identity.person_id) {
						if person.org_id == org_id {
							return Ok(Some(identity.clone()));
						}
					}
				}
			}
			Ok(None)
		}

		async fn list_identities_for_person(&self, person_id: PersonId) -> Result<Vec<PersonIdentity>> {
			let identities = self.identities.lock().unwrap();
			let result: Vec<PersonIdentity> = identities
				.values()
				.filter(|i| i.person_id == person_id)
				.cloned()
				.collect();
			Ok(result)
		}

		async fn transfer_identities(
			&self,
			from_person_id: PersonId,
			to_person_id: PersonId,
		) -> Result<u64> {
			let mut identities = self.identities.lock().unwrap();
			let mut count = 0u64;

			for identity in identities.values_mut() {
				if identity.person_id == from_person_id {
					identity.person_id = to_person_id;
					count += 1;
				}
			}
			Ok(count)
		}

		async fn insert_event(&self, event: &Event) -> Result<()> {
			self.events.lock().unwrap().insert(event.id, event.clone());
			Ok(())
		}

		async fn insert_events(&self, events: &[Event]) -> Result<u64> {
			let mut store = self.events.lock().unwrap();
			for event in events {
				store.insert(event.id, event.clone());
			}
			Ok(events.len() as u64)
		}

		async fn get_event_by_id(&self, id: EventId) -> Result<Option<Event>> {
			Ok(self.events.lock().unwrap().get(&id).cloned())
		}

		async fn list_events(
			&self,
			org_id: OrgId,
			distinct_id: Option<&str>,
			event_name: Option<&str>,
			_start_time: Option<DateTime<Utc>>,
			_end_time: Option<DateTime<Utc>>,
			limit: u32,
			offset: u32,
		) -> Result<Vec<Event>> {
			let events = self.events.lock().unwrap();
			let result: Vec<Event> = events
				.values()
				.filter(|e| {
					e.org_id == org_id
						&& distinct_id.map_or(true, |d| e.distinct_id == d)
						&& event_name.map_or(true, |n| e.event_name == n)
				})
				.skip(offset as usize)
				.take(limit as usize)
				.cloned()
				.collect();
			Ok(result)
		}

		async fn count_events(
			&self,
			org_id: OrgId,
			distinct_id: Option<&str>,
			event_name: Option<&str>,
			_start_time: Option<DateTime<Utc>>,
			_end_time: Option<DateTime<Utc>>,
		) -> Result<u64> {
			let events = self.events.lock().unwrap();
			let count = events
				.values()
				.filter(|e| {
					e.org_id == org_id
						&& distinct_id.map_or(true, |d| e.distinct_id == d)
						&& event_name.map_or(true, |n| e.event_name == n)
				})
				.count();
			Ok(count as u64)
		}

		async fn reassign_events(
			&self,
			from_person_id: PersonId,
			to_person_id: PersonId,
		) -> Result<u64> {
			let mut events = self.events.lock().unwrap();
			let mut count = 0u64;

			for event in events.values_mut() {
				if event.person_id == Some(from_person_id) {
					event.person_id = Some(to_person_id);
					count += 1;
				}
			}
			Ok(count)
		}

		async fn create_merge(&self, merge: &PersonMerge) -> Result<()> {
			self.merges.lock().unwrap().push(merge.clone());
			Ok(())
		}

		async fn list_merges_for_person(&self, person_id: PersonId) -> Result<Vec<PersonMerge>> {
			let merges = self.merges.lock().unwrap();
			let result: Vec<PersonMerge> = merges
				.iter()
				.filter(|m| m.winner_id == person_id || m.loser_id == person_id)
				.cloned()
				.collect();
			Ok(result)
		}

		async fn create_api_key(&self, _key: &AnalyticsApiKey) -> Result<()> {
			Ok(())
		}

		async fn get_api_key_by_id(&self, _id: AnalyticsApiKeyId) -> Result<Option<AnalyticsApiKey>> {
			Ok(None)
		}

		async fn get_api_key_by_hash(&self, _key_hash: &str) -> Result<Option<AnalyticsApiKey>> {
			Ok(None)
		}

		async fn list_api_keys(&self, _org_id: OrgId) -> Result<Vec<AnalyticsApiKey>> {
			Ok(vec![])
		}

		async fn revoke_api_key(&self, _id: AnalyticsApiKeyId) -> Result<bool> {
			Ok(false)
		}

		async fn update_api_key_last_used(&self, _id: AnalyticsApiKeyId) -> Result<()> {
			Ok(())
		}

		async fn find_api_key_by_verification(
			&self,
			_raw_key: &str,
			_org_id: OrgId,
		) -> Result<Option<AnalyticsApiKey>> {
			Ok(None)
		}

		async fn find_api_key_by_raw(&self, _raw_key: &str) -> Result<Option<AnalyticsApiKey>> {
			Ok(None)
		}
	}

	#[tokio::test]
	async fn resolve_person_creates_new_person() {
		let repo = MockRepository::new();
		let service = IdentityResolutionService::new(repo.clone());
		let org_id = OrgId::new();

		let result = service
			.resolve_person_for_distinct_id(org_id, "anon_123")
			.await
			.unwrap();

		assert_eq!(result.person.org_id, org_id);
		assert_eq!(result.identities.len(), 1);
		assert_eq!(result.identities[0].distinct_id, "anon_123");
		assert!(result.identities[0].is_anonymous());
	}

	#[tokio::test]
	async fn resolve_person_returns_existing_person() {
		let repo = MockRepository::new();
		let service = IdentityResolutionService::new(repo.clone());
		let org_id = OrgId::new();

		let first = service
			.resolve_person_for_distinct_id(org_id, "anon_123")
			.await
			.unwrap();

		let second = service
			.resolve_person_for_distinct_id(org_id, "anon_123")
			.await
			.unwrap();

		assert_eq!(first.person.id, second.person.id);
	}

	#[tokio::test]
	async fn identify_creates_new_person_when_both_unknown() {
		let repo = MockRepository::new();
		let service = IdentityResolutionService::new(repo.clone());
		let org_id = OrgId::new();

		let payload = IdentifyPayload::new("anon_123".to_string(), "user@example.com".to_string())
			.with_properties(serde_json::json!({"plan": "pro"}));

		let result = service.identify(org_id, payload).await.unwrap();

		assert_eq!(result.identities.len(), 2);
		assert!(result.has_identified_identity());
		assert_eq!(result.person.properties["plan"], "pro");
	}

	#[tokio::test]
	async fn identify_links_user_to_existing_anonymous_person() {
		let repo = MockRepository::new();
		let service = IdentityResolutionService::new(repo.clone());
		let org_id = OrgId::new();

		let anon_person = service
			.resolve_person_for_distinct_id(org_id, "anon_123")
			.await
			.unwrap();

		let payload = IdentifyPayload::new("anon_123".to_string(), "user@example.com".to_string());

		let result = service.identify(org_id, payload).await.unwrap();

		assert_eq!(result.person.id, anon_person.person.id);
		assert_eq!(result.identities.len(), 2);
		assert!(result.has_identified_identity());
	}

	#[tokio::test]
	async fn identify_links_anon_to_existing_identified_person() {
		let repo = MockRepository::new();
		let service = IdentityResolutionService::new(repo.clone());
		let org_id = OrgId::new();

		let existing = IdentifyPayload::new("old_anon".to_string(), "user@example.com".to_string());
		let existing_person = service.identify(org_id, existing).await.unwrap();

		let payload = IdentifyPayload::new("new_anon".to_string(), "user@example.com".to_string());
		let result = service.identify(org_id, payload).await.unwrap();

		assert_eq!(result.person.id, existing_person.person.id);
		assert!(result
			.identities
			.iter()
			.any(|i| i.distinct_id == "new_anon"));
	}

	#[tokio::test]
	async fn identify_merges_two_different_persons() {
		let repo = MockRepository::new();
		let service = IdentityResolutionService::new(repo.clone());
		let org_id = OrgId::new();

		let _person_a = service
			.resolve_person_for_distinct_id(org_id, "anon_a")
			.await
			.unwrap();
		let _person_b = service
			.resolve_person_for_distinct_id(org_id, "anon_b")
			.await
			.unwrap();

		let identify_a = IdentifyPayload::new("anon_a".to_string(), "user_a@example.com".to_string());
		let identified_a = service.identify(org_id, identify_a).await.unwrap();

		let payload = IdentifyPayload::new("anon_b".to_string(), "user_a@example.com".to_string());
		let result = service.identify(org_id, payload).await.unwrap();

		assert_eq!(result.person.id, identified_a.person.id);

		let merges = repo.merges.lock().unwrap();
		assert_eq!(merges.len(), 1);
		assert_eq!(merges[0].winner_id, identified_a.person.id);
	}

	#[tokio::test]
	async fn alias_creates_new_person_when_both_unknown() {
		let repo = MockRepository::new();
		let service = IdentityResolutionService::new(repo.clone());
		let org_id = OrgId::new();

		let payload = AliasPayload::new("primary".to_string(), "secondary".to_string());
		let result = service.alias(org_id, payload).await.unwrap();

		assert_eq!(result.identities.len(), 2);
		assert!(result.identities.iter().any(|i| i.distinct_id == "primary"));
		assert!(result
			.identities
			.iter()
			.any(|i| i.distinct_id == "secondary"));
	}

	#[tokio::test]
	async fn alias_links_to_existing_primary_person() {
		let repo = MockRepository::new();
		let service = IdentityResolutionService::new(repo.clone());
		let org_id = OrgId::new();

		let existing = service
			.resolve_person_for_distinct_id(org_id, "primary")
			.await
			.unwrap();

		let payload = AliasPayload::new("primary".to_string(), "secondary".to_string());
		let result = service.alias(org_id, payload).await.unwrap();

		assert_eq!(result.person.id, existing.person.id);
		assert_eq!(result.identities.len(), 2);
	}

	#[tokio::test]
	async fn alias_merges_two_different_persons() {
		let repo = MockRepository::new();
		let service = IdentityResolutionService::new(repo.clone());
		let org_id = OrgId::new();

		let person_a = service
			.resolve_person_for_distinct_id(org_id, "id_a")
			.await
			.unwrap();
		let _person_b = service
			.resolve_person_for_distinct_id(org_id, "id_b")
			.await
			.unwrap();

		let payload = AliasPayload::new("id_a".to_string(), "id_b".to_string());
		let result = service.alias(org_id, payload).await.unwrap();

		assert_eq!(result.person.id, person_a.person.id);

		let merges = repo.merges.lock().unwrap();
		assert_eq!(merges.len(), 1);
	}

	#[tokio::test]
	async fn merge_prefers_identified_person() {
		let repo = MockRepository::new();
		let service = IdentityResolutionService::new(repo.clone());
		let org_id = OrgId::new();

		let _anon_person = service
			.resolve_person_for_distinct_id(org_id, "anon_123")
			.await
			.unwrap();

		let identify_payload =
			IdentifyPayload::new("other_anon".to_string(), "user@example.com".to_string());
		let identified_person = service.identify(org_id, identify_payload).await.unwrap();

		let alias_payload = AliasPayload::new("anon_123".to_string(), "user@example.com".to_string());
		let result = service.alias(org_id, alias_payload).await.unwrap();

		assert_eq!(result.person.id, identified_person.person.id);
	}

	#[tokio::test]
	async fn merge_transfers_events() {
		let repo = MockRepository::new();
		let service = IdentityResolutionService::new(repo.clone());
		let org_id = OrgId::new();

		let person_a = service
			.resolve_person_for_distinct_id(org_id, "id_a")
			.await
			.unwrap();
		let person_b = service
			.resolve_person_for_distinct_id(org_id, "id_b")
			.await
			.unwrap();

		let mut event = Event::new(org_id, "id_b".to_string(), "test_event".to_string());
		event.person_id = Some(person_b.person.id);
		repo.insert_event(&event).await.unwrap();

		let payload = AliasPayload::new("id_a".to_string(), "id_b".to_string());
		let _result = service.alias(org_id, payload).await.unwrap();

		let stored_event = repo.get_event_by_id(event.id).await.unwrap().unwrap();
		assert_eq!(stored_event.person_id, Some(person_a.person.id));
	}

	#[tokio::test]
	async fn merge_preserves_winner_properties() {
		let repo = MockRepository::new();
		let org_id = OrgId::new();

		let person_a = Person::new(org_id).with_properties(serde_json::json!({
			"name": "Alice",
			"plan": "free"
		}));
		repo.create_person(&person_a).await.unwrap();
		let identity_a = PersonIdentity::anonymous(person_a.id, "id_a".to_string());
		repo.create_identity(&identity_a).await.unwrap();

		let person_b = Person::new(org_id).with_properties(serde_json::json!({
			"plan": "pro",
			"email": "bob@example.com"
		}));
		repo.create_person(&person_b).await.unwrap();
		let identity_b = PersonIdentity::anonymous(person_b.id, "id_b".to_string());
		repo.create_identity(&identity_b).await.unwrap();

		let service = IdentityResolutionService::new(repo.clone());

		let payload = AliasPayload::new("id_a".to_string(), "id_b".to_string());
		let result = service.alias(org_id, payload).await.unwrap();

		assert_eq!(result.person.properties["name"], "Alice");
		assert_eq!(result.person.properties["plan"], "free");
		assert_eq!(result.person.properties["email"], "bob@example.com");
	}

	#[tokio::test]
	async fn same_person_identify_updates_properties() {
		let repo = MockRepository::new();
		let service = IdentityResolutionService::new(repo.clone());
		let org_id = OrgId::new();

		let first_payload =
			IdentifyPayload::new("anon_123".to_string(), "user@example.com".to_string())
				.with_properties(serde_json::json!({"plan": "free"}));
		let first_result = service.identify(org_id, first_payload).await.unwrap();

		let second_payload =
			IdentifyPayload::new("anon_123".to_string(), "user@example.com".to_string())
				.with_properties(serde_json::json!({"plan": "pro", "name": "Alice"}));
		let second_result = service.identify(org_id, second_payload).await.unwrap();

		assert_eq!(first_result.person.id, second_result.person.id);
		assert_eq!(second_result.person.properties["plan"], "pro");
		assert_eq!(second_result.person.properties["name"], "Alice");
	}

	#[tokio::test]
	async fn same_person_alias_is_noop() {
		let repo = MockRepository::new();
		let service = IdentityResolutionService::new(repo.clone());
		let org_id = OrgId::new();

		let alias_payload = AliasPayload::new("id_a".to_string(), "id_b".to_string());
		let first = service.alias(org_id, alias_payload).await.unwrap();

		let alias_again = AliasPayload::new("id_a".to_string(), "id_b".to_string());
		let second = service.alias(org_id, alias_again).await.unwrap();

		assert_eq!(first.person.id, second.person.id);

		let merges = repo.merges.lock().unwrap();
		assert_eq!(merges.len(), 0);
	}
}
