// Copyright 2019 The Exonum Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use exonum_merkledb::{Fork, Snapshot};
use failure::Error;
use protobuf::well_known_types::Any;

use std::fmt::Debug;

use crate::{
    api::ServiceApiBuilder,
    crypto::Hash,
    messages::MethodId,
    runtime::{error::ExecutionError, rust::TransactionContext},
};

use super::RustArtifactSpec;

pub trait ServiceDispatcher {
    fn call(
        &self,
        method: MethodId,
        ctx: TransactionContext,
        payload: &[u8],
    ) -> Result<Result<(), ExecutionError>, Error>;
}

pub trait Service: ServiceDispatcher + Debug + 'static {
    fn initialize(&mut self, _ctx: TransactionContext, _arg: &Any) -> Result<(), ExecutionError> {
        Ok(())
    }

    fn before_commit(&self, _fork: &Fork) {}

    fn after_commit(&self, _fork: &Fork) {}

    fn state_hash(&self, _snapshot: &dyn Snapshot) -> Vec<Hash> {
        vec![]
    }

    fn wire_api(&self, _builder: &mut ServiceApiBuilder) {}
    // TODO: add other hooks such as "on node startup", etc.
}

pub trait ServiceFactory: Debug + 'static {
    fn artifact(&self) -> RustArtifactSpec;
    fn new_instance(&self) -> Box<dyn Service>;
}

impl<T> From<T> for Box<dyn ServiceFactory>
where
    T: ServiceFactory,
{
    fn from(factory: T) -> Self {
        Box::new(factory) as Self
    }
}

#[macro_export]
macro_rules! impl_service_dispatcher {
    ($struct_name:ident, $interface:ident) => {
        impl $crate::runtime::rust::service::ServiceDispatcher for $struct_name {
            fn call(
                &self,
                method: $crate::messages::MethodId,
                ctx: $crate::runtime::rust::TransactionContext,
                payload: &[u8],
            ) -> Result<Result<(), $crate::runtime::error::ExecutionError>, failure::Error> {
                <$struct_name as $interface>::_dispatch(self, ctx, method, payload)
            }
        }
    };
}