use core::ptr::NonNull;

use crate::{ArtifactContract, ArtifactEndpoint, ArtifactOpTag, ArtifactOutputDescriptor, PathgenError};

/// Alignment-pinned descriptor used by the tagged operation handle.
///
/// `repr(align(8))` reserves three low pointer bits for the operation family.
/// This mirrors the uploaded tagged_ptr_core pattern: keep the clean pointer for
/// provenance, keep the tag separately, and only expose the packed word for
/// evidence/comparison/queues. The public API never dereferences a tagged word.
#[repr(align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArtifactOpDescriptor {
    pub output: ArtifactOutputDescriptor,
}

impl ArtifactOpDescriptor {
    pub const fn new(output: ArtifactOutputDescriptor) -> Self {
        Self { output }
    }
}

/// Compiled HKT-like operation handle:
///
/// ```text
/// descriptor pointer + endpoint tag = applied artifact output operation
/// ```
///
/// The descriptor is still the authority. The tag is only the fast operation
/// family selector for writer/codec dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArtifactOpHandle {
    ptr: NonNull<ArtifactOpDescriptor>,
    tag: ArtifactOpTag,
}

impl ArtifactOpHandle {
    pub fn new(descriptor: &'static ArtifactOpDescriptor) -> Result<Self, PathgenError> {
        let addr = descriptor as *const ArtifactOpDescriptor as usize;
        if addr & ArtifactOpTag::MASK != 0 {
            return Err(PathgenError::MisalignedArtifactOpDescriptor {
                addr,
                align: 1usize << ArtifactOpTag::LOW_BITS,
            });
        }

        Ok(Self {
            ptr: NonNull::new(descriptor as *const ArtifactOpDescriptor as *mut ArtifactOpDescriptor)
                .expect("static descriptor pointer cannot be null"),
            tag: ArtifactOpTag::from_endpoint(descriptor.output.endpoint),
        })
    }

    /// Compile and validate an export operation for an artifact contract.
    pub fn compile_export<A: ArtifactContract>(
        descriptor: &'static ArtifactOpDescriptor,
    ) -> Result<Self, PathgenError> {
        validate_contract::<A>(descriptor)?;
        validate_endpoint_declared(A::EXPORTS, descriptor.output.endpoint, "exports")?;
        Self::new(descriptor)
    }

    /// Compile and validate an import operation for an artifact contract.
    pub fn compile_import<A: ArtifactContract>(
        descriptor: &'static ArtifactOpDescriptor,
    ) -> Result<Self, PathgenError> {
        validate_contract::<A>(descriptor)?;
        validate_endpoint_declared(A::IMPORTS, descriptor.output.endpoint, "imports")?;
        Self::new(descriptor)
    }

    pub const fn tag(self) -> ArtifactOpTag {
        self.tag
    }

    /// Return the clean static descriptor pointer. This is safe because handles
    /// can only be constructed from `&'static ArtifactOpDescriptor` through the
    /// safe constructors above.
    pub fn descriptor(self) -> &'static ArtifactOpDescriptor {
        // SAFETY: `self.ptr` came from a `&'static ArtifactOpDescriptor` in
        // `new`, and the handle never reconstructs a pointer from an integer.
        unsafe { self.ptr.as_ref() }
    }

    pub fn endpoint(self) -> ArtifactEndpoint {
        self.descriptor().output.endpoint
    }

    /// Packed evidence word. Low three bits contain [`ArtifactOpTag`]; upper
    /// bits are the clean descriptor address. Do not dereference this word.
    pub fn word(self) -> usize {
        (self.ptr.as_ptr() as usize) | self.tag.as_usize()
    }

    pub fn clean_addr(self) -> usize {
        self.ptr.as_ptr() as usize
    }
}

fn validate_contract<A: ArtifactContract>(descriptor: &ArtifactOpDescriptor) -> Result<(), PathgenError> {
    let output = descriptor.output;

    if output.route_id != A::ROUTE_ID {
        return Err(PathgenError::ArtifactContractMismatch {
            field: "route_id",
            expected: A::ROUTE_ID,
            actual: output.route_id,
        });
    }

    if output.schema != A::SCHEMA {
        return Err(PathgenError::ArtifactContractMismatch {
            field: "schema",
            expected: A::SCHEMA,
            actual: output.schema,
        });
    }

    if output.kind != A::KIND {
        return Err(PathgenError::ArtifactContractMismatch {
            field: "kind",
            expected: A::KIND,
            actual: output.kind,
        });
    }

    if output.version != A::VERSION {
        return Err(PathgenError::ArtifactVersionMismatch {
            expected: A::VERSION,
            actual: output.version,
        });
    }

    Ok(())
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
