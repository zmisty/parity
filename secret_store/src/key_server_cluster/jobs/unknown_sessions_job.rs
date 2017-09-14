// Copyright 2015-2017 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

use std::sync::Arc;
use std::collections::{BTreeSet, BTreeMap};
use ethkey::{Public, Secret};
use ethcrypto::ecies::encrypt;
use ethcrypto::DEFAULT_MAC;
use key_server_cluster::{Error, NodeId, SessionId, DocumentKeyShare, EncryptedDocumentKeyShadow, KeyStorage};
use key_server_cluster::math;
use key_server_cluster::jobs::job_session::{JobPartialRequestAction, JobPartialResponseAction, JobExecutor};

/// Unknown sessions report job.
pub struct UnknownSessionsJob<'a> {
	/// Target node id.
	target_node_id: Option<NodeId>,
	/// Keys storage.
	key_storage: Option<Arc<KeyStorage + 'a>>,
}

impl<'a> UnknownSessionsJob<'a> {
	pub fn new_on_slave(key_storage: Arc<KeyStorage + 'a>) -> Self {
		UnknownSessionsJob {
			target_node_id: None,
			key_storage: Some(key_storage),
		}
	}

	pub fn new_on_master(self_node_id: NodeId) -> Self {
		UnknownSessionsJob {
			target_node_id: Some(self_node_id),
			key_storage: None,
		}
	}
}

impl<'a> JobExecutor for UnknownSessionsJob<'a> {
	type PartialJobRequest = NodeId;
	type PartialJobResponse = BTreeSet<SessionId>;
	type JobResponse = BTreeMap<SessionId, BTreeSet<NodeId>>;

	fn prepare_partial_request(&self, _node: &NodeId, _nodes: &BTreeSet<NodeId>) -> Result<NodeId, Error> {
		Ok(self.target_node_id.clone().expect("prepare_partial_request is only called on master nodes; this field is filled on master nodes in constructor; qed"))
	}

	fn process_partial_request(&self, partial_request: NodeId) -> Result<JobPartialRequestAction<BTreeSet<SessionId>>, Error> {
		let key_storage = self.key_storage.as_ref().expect("process_partial_request is only called on slave nodes; this field is filled on slave nodes in constructor; qed");
		Ok(JobPartialRequestAction::Respond(key_storage.iter()
			.filter(|&(_, ref key_share)| !key_share.id_numbers.contains_key(&partial_request))
			.map(|(id, _)| id.clone())
			.collect()))
	}

	fn check_partial_response(&self, _partial_response: &BTreeSet<SessionId>) -> Result<JobPartialResponseAction, Error> {
		Ok(JobPartialResponseAction::Accept)
	}

	// TODO: add partial response computation + partial-partial responses
	fn compute_response(&self, partial_responses: &BTreeMap<NodeId, BTreeSet<SessionId>>) -> Result<BTreeMap<SessionId, BTreeSet<NodeId>>, Error> {
		let mut result: BTreeMap<SessionId, BTreeSet<NodeId>> = BTreeMap::new();
		for (node_id, node_sessions) in partial_responses {
			for node_session in node_sessions {
				result.entry(node_session.clone())
					.or_insert_with(Default::default)
					.insert(node_id.clone());
			}
		}

		Ok(result)
	}
}