use std::fmt;

/// Defines the type of modification applied to the organism
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModificationType {
    /// A new organ is added (e.g., new LXC)
    StructuralAddition,
    /// An organ is removed (Garbage Collection)
    StructuralRemoval,
    /// Minor configuration tweak (RAM change, etc.)
    MetabolicTweak,
    /// Unchanged
    None,
}

/// Domain Entity: Version of the Organism or Organ
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganismVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl OrganismVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Automatically increment the version based on the biological modification
    pub fn increment(&mut self, mod_type: ModificationType) {
        match mod_type {
            ModificationType::StructuralAddition | ModificationType::StructuralRemoval => {
                // Structural changes bump the minor version (evolution)
                self.minor += 1;
                self.patch = 0;
            }
            ModificationType::MetabolicTweak => {
                // Minor tweaks bump the patch version (adaptation)
                self.patch += 1;
            }
            ModificationType::None => {}
        }
    }
}

impl fmt::Display for OrganismVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
