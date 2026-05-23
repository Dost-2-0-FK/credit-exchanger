use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::domain::credit::Credit;

#[derive(Serialize, Deserialize)]
pub(crate) struct BlocUser {
	pub(crate) resources: HashMap<String, Credit>,
}

impl BlocUser {
	pub(crate) fn from_domain(user: &crate::domain::bloc_user::BlocUser) -> Self {
		Self {
			resources: user.resources().clone(),
		}
	}
}
