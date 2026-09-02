use crate::{
    ArtifactContract, ArtifactEndpoint, ArtifactOpDescriptor, ArtifactOpHandle, PathgenError,
};

/// Direction requested when compiling a registered artifact operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactOpDirection {
    Import,
    Export,
}

/// Static registry of declared artifact operation descriptors.
///
/// The registry is the validation layer between durable artifact contracts and
/// the compact tagged operation handles:
///
/// ```text
/// ArtifactContract + requested endpoint
///   -> registry lookup
///   -> contract/endpoint validation
///   -> ArtifactOpHandle
/// ```
///
/// The tagged handle is therefore a compiled dispatch token, not the source of
/// truth. Descriptors in this registry remain the durable authority.
#[derive(Debug, Clone, Copy)]
pub struct ArtifactOpRegistry {
    descriptors: &'static [ArtifactOpDescriptor],
}

impl ArtifactOpRegistry {
    pub const fn new(descriptors: &'static [ArtifactOpDescriptor]) -> Self {
        Self { descriptors }
    }

    pub const fn empty() -> Self {
        Self { descriptors: &[] }
    }

    pub const fn descriptors(self) -> &'static [ArtifactOpDescriptor] {
        self.descriptors
    }

    pub const fn len(self) -> usize {
        self.descriptors.len()
    }

    pub const fn is_empty(self) -> bool {
        self.descriptors.is_empty()
    }

    /// Validate that the registry has no duplicate route+endpoint bindings.
    /// Duplicate bindings are rejected because they would make dispatch and
    /// receipts ambiguous.
    pub fn validate_unique(self) -> Result<(), PathgenError> {
        for (idx, left) in self.descriptors.iter().enumerate() {
            for right in &self.descriptors[idx + 1..] {
                if same_binding(left, right) {
                    return Err(PathgenError::DuplicateArtifactOpDescriptor {
                        route_id: left.output.route_id,
                        endpoint: left.output.endpoint.label(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Find a descriptor by its semantic route and endpoint.
    pub fn find(
        self,
        route_id: &str,
        endpoint: ArtifactEndpoint,
    ) -> Result<&'static ArtifactOpDescriptor, PathgenError> {
        let mut found: Option<&'static ArtifactOpDescriptor> = None;

        for descriptor in self.descriptors {
            if descriptor.output.route_id == route_id && descriptor.output.endpoint == endpoint {
                if found.is_some() {
                    return Err(PathgenError::DuplicateArtifactOpDescriptor {
                        route_id: descriptor.output.route_id,
                        endpoint: descriptor.output.endpoint.label(),
                    });
                }

                found = Some(descriptor);
            }
        }

        found.ok_or_else(|| PathgenError::ArtifactOpDescriptorNotRegistered {
            route_id: route_id.to_string(),
            endpoint: endpoint.label(),
        })
    }

    pub fn contains(self, route_id: &str, endpoint: ArtifactEndpoint) -> bool {
        self.find(route_id, endpoint).is_ok()
    }

    /// Compile an export operation for an artifact contract.
    ///
    /// This first checks the contract's declared export endpoints, then looks up
    /// the matching registered descriptor, and finally compiles the safe tagged
    /// operation handle.
    pub fn compile_export<A: ArtifactContract>(
        self,
        endpoint: ArtifactEndpoint,
    ) -> Result<ArtifactOpHandle, PathgenError> {
        validate_endpoint_declared(A::EXPORTS, endpoint, "exports")?;
        let descriptor = self.find(A::ROUTE_ID, endpoint)?;
        ArtifactOpHandle::compile_export::<A>(descriptor)
    }

    /// Compile an import operation for an artifact contract.
    pub fn compile_import<A: ArtifactContract>(
        self,
        endpoint: ArtifactEndpoint,
    ) -> Result<ArtifactOpHandle, PathgenError> {
        validate_endpoint_declared(A::IMPORTS, endpoint, "imports")?;
        let descriptor = self.find(A::ROUTE_ID, endpoint)?;
        ArtifactOpHandle::compile_import::<A>(descriptor)
    }

    /// Compile the canonical endpoint as an export operation.
    pub fn compile_canonical_export<A: ArtifactContract>(
        self,
    ) -> Result<ArtifactOpHandle, PathgenError> {
        self.compile_export::<A>(A::CANONICAL)
    }

    /// Compile the canonical endpoint as an import operation.
    pub fn compile_canonical_import<A: ArtifactContract>(
        self,
    ) -> Result<ArtifactOpHandle, PathgenError> {
        self.compile_import::<A>(A::CANONICAL)
    }

    pub fn compile<A: ArtifactContract>(
        self,
        direction: ArtifactOpDirection,
        endpoint: ArtifactEndpoint,
    ) -> Result<ArtifactOpHandle, PathgenError> {
        match direction {
            ArtifactOpDirection::Import => self.compile_import::<A>(endpoint),
            ArtifactOpDirection::Export => self.compile_export::<A>(endpoint),
        }
    }
}

fn same_binding(left: &ArtifactOpDescriptor, right: &ArtifactOpDescriptor) -> bool {
    left.output.route_id == right.output.route_id && left.output.endpoint == right.output.endpoint
}

fn validate_endpoint_declared(
    declared: &'static [ArtifactEndpoint],
    endpoint: ArtifactEndpoint,
    direction: &'static str,
) -> Result<(), PathgenError> {
    if declared.contains(&endpoint) {
        Ok(())
    } else {
        Err(PathgenError::ArtifactEndpointNotDeclared {
            endpoint: endpoint.label(),
            direction,
        })
    }
}
