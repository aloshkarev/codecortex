//! Refactoring recommendation engine
//!
//! This module maps detected code smells to recommended refactoring techniques
//! from the refactoring.guru catalog.

use crate::{CodeSmell, Severity, SmellCategory, SmellType};
use serde::{Deserialize, Serialize};

/// Priority level for refactoring recommendations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Priority {
    /// Low priority - cosmetic or minor improvement
    Low = 1,
    /// Medium priority - should be addressed soon
    Medium = 2,
    /// High priority - important for maintainability
    High = 3,
    /// Critical - blocking or severe issues
    Critical = 4,
}

impl Default for Priority {
    fn default() -> Self {
        Self::Medium
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

/// Refactoring technique from refactoring.guru catalog
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefactoringTechnique {
    // === Composing Methods ===
    /// Turn a code fragment into a method whose name explains its purpose
    ExtractMethod,
    /// Replace method calls with the method's body
    InlineMethod,
    /// Replace expression with a variable
    ExtractVariable,
    /// Replace variable with its expression
    InlineVariable,
    /// Replace temp variable with a query method
    ReplaceTempWithQuery,
    /// Split variable used for multiple purposes
    SplitTemporaryVariable,
    /// Remove assignments to parameters
    RemoveAssignmentsToParameters,
    /// Replace method with an object
    ReplaceMethodWithMethodObject,
    /// Substitute algorithm with clearer one
    SubstituteAlgorithm,

    // === Moving Features Between Objects ===
    /// Move method to another class
    MoveMethod,
    /// Move field to another class
    MoveField,
    /// Extract fields and methods into a new class
    ExtractClass,
    /// Merge class into another
    InlineClass,
    /// Hide delegate by creating delegating methods
    HideDelegate,
    /// Remove simple delegating methods
    RemoveMiddleMan,
    /// Add method to a class you can't modify
    IntroduceForeignMethod,
    /// Add extension methods to a class
    IntroduceLocalExtension,

    // === Organizing Data ===
    /// Replace magic literal with constant
    ReplaceMagicLiteral,
    /// Convert value to reference object
    ChangeValueToReference,
    /// Convert reference to value object
    ChangeReferenceToValue,
    /// Replace array with object
    ReplaceArrayWithObject,
    /// Replace type code with strategy pattern
    ReplaceTypeCodeWithStrategy,
    /// Replace type code with state pattern
    ReplaceTypeCodeWithState,
    /// Replace type code with subclass
    ReplaceTypeCodeWithSubclass,
    /// Replace subclass with fields
    ReplaceSubclassWithFields,

    // === Simplifying Conditional Expressions ===
    /// Break complex conditional into smaller pieces
    DecomposeConditional,
    /// Combine multiple conditionals with same result
    ConsolidateConditional,
    /// Replace nested conditionals with guard clauses
    ReplaceNestedConditionalWithGuard,
    /// Replace conditional with polymorphism
    ReplaceConditionalWithPolymorphism,
    /// Introduce null object
    IntroduceNullObject,
    /// Introduce assertion
    IntroduceAssertion,

    // === Simplifying Method Calls ===
    /// Rename method to better reflect purpose
    RenameMethod,
    /// Add parameter for new functionality
    AddParameter,
    /// Remove unused parameter
    RemoveParameter,
    /// Combine parameters into object
    IntroduceParameterObject,
    /// Pass whole object instead of extracting parts
    PreserveWholeObject,
    /// Remove setter method
    RemoveSettingMethod,
    /// Hide method from public interface
    HideMethod,
    /// Replace constructor with factory method
    ReplaceConstructorWithFactory,
    /// Replace error code with exception
    ReplaceErrorCodeWithException,
    /// Replace exception with test
    ReplaceExceptionWithTest,

    // === Dealing with Generalization ===
    /// Move field to superclass
    PullUpField,
    /// Move method to superclass
    PullUpMethod,
    /// Move field to subclass
    PushDownField,
    /// Move method to subclass
    PushDownMethod,
    /// Extract subclass for specialized behavior
    ExtractSubclass,
    /// Extract interface from implementation
    ExtractInterface,
    /// Merge hierarchy into single class
    CollapseHierarchy,
    /// Create template method pattern
    FormTemplateMethod,
    /// Replace inheritance with delegation
    ReplaceInheritanceWithDelegation,
    /// Replace delegation with inheritance
    ReplaceDelegationWithInheritance,

    // === Large-Scale Refactorings ===
    /// Encapsulate field with getter/setter
    EncapsulateField,
    /// Unify interfaces
    UnifyInterfaces,
    /// Extract hierarchy
    ExtractHierarchy,
}

impl std::fmt::Display for RefactoringTechnique {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Convert CamelCase to snake_case
        let name = match self {
            Self::ExtractMethod => "Extract Method",
            Self::InlineMethod => "Inline Method",
            Self::ExtractVariable => "Extract Variable",
            Self::InlineVariable => "Inline Variable",
            Self::ReplaceTempWithQuery => "Replace Temp with Query",
            Self::SplitTemporaryVariable => "Split Temporary Variable",
            Self::RemoveAssignmentsToParameters => "Remove Assignments to Parameters",
            Self::ReplaceMethodWithMethodObject => "Replace Method with Method Object",
            Self::SubstituteAlgorithm => "Substitute Algorithm",
            Self::MoveMethod => "Move Method",
            Self::MoveField => "Move Field",
            Self::ExtractClass => "Extract Class",
            Self::InlineClass => "Inline Class",
            Self::HideDelegate => "Hide Delegate",
            Self::RemoveMiddleMan => "Remove Middle Man",
            Self::IntroduceForeignMethod => "Introduce Foreign Method",
            Self::IntroduceLocalExtension => "Introduce Local Extension",
            Self::ReplaceMagicLiteral => "Replace Magic Literal",
            Self::ChangeValueToReference => "Change Value to Reference",
            Self::ChangeReferenceToValue => "Change Reference to Value",
            Self::ReplaceArrayWithObject => "Replace Array with Object",
            Self::ReplaceTypeCodeWithStrategy => "Replace Type Code with Strategy",
            Self::ReplaceTypeCodeWithState => "Replace Type Code with State",
            Self::ReplaceTypeCodeWithSubclass => "Replace Type Code with Subclass",
            Self::ReplaceSubclassWithFields => "Replace Subclass with Fields",
            Self::DecomposeConditional => "Decompose Conditional",
            Self::ConsolidateConditional => "Consolidate Conditional",
            Self::ReplaceNestedConditionalWithGuard => "Replace Nested Conditional with Guard",
            Self::ReplaceConditionalWithPolymorphism => "Replace Conditional with Polymorphism",
            Self::IntroduceNullObject => "Introduce Null Object",
            Self::IntroduceAssertion => "Introduce Assertion",
            Self::RenameMethod => "Rename Method",
            Self::AddParameter => "Add Parameter",
            Self::RemoveParameter => "Remove Parameter",
            Self::IntroduceParameterObject => "Introduce Parameter Object",
            Self::PreserveWholeObject => "Preserve Whole Object",
            Self::RemoveSettingMethod => "Remove Setting Method",
            Self::HideMethod => "Hide Method",
            Self::ReplaceConstructorWithFactory => "Replace Constructor with Factory",
            Self::ReplaceErrorCodeWithException => "Replace Error Code with Exception",
            Self::ReplaceExceptionWithTest => "Replace Exception with Test",
            Self::PullUpField => "Pull Up Field",
            Self::PullUpMethod => "Pull Up Method",
            Self::PushDownField => "Push Down Field",
            Self::PushDownMethod => "Push Down Method",
            Self::ExtractSubclass => "Extract Subclass",
            Self::ExtractInterface => "Extract Interface",
            Self::CollapseHierarchy => "Collapse Hierarchy",
            Self::FormTemplateMethod => "Form Template Method",
            Self::ReplaceInheritanceWithDelegation => "Replace Inheritance with Delegation",
            Self::ReplaceDelegationWithInheritance => "Replace Delegation with Inheritance",
            Self::EncapsulateField => "Encapsulate Field",
            Self::UnifyInterfaces => "Unify Interfaces",
            Self::ExtractHierarchy => "Extract Hierarchy",
        };
        write!(f, "{}", name)
    }
}

/// A refactoring recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringRecommendation {
    /// The code smell this recommendation addresses
    pub smell_type: SmellType,
    /// The refactoring technique to apply
    pub technique: RefactoringTechnique,
    /// Priority level for this refactoring
    pub priority: Priority,
    /// Brief description of why this refactoring is needed
    pub description: String,
    /// Step-by-step guide for applying this refactoring
    pub steps: Vec<String>,
    /// Related smells that might be addressed simultaneously
    pub related_smells: Vec<SmellType>,
}

impl RefactoringRecommendation {
    /// Create a new refactoring recommendation
    pub fn new(smell_type: SmellType, technique: RefactoringTechnique, priority: Priority) -> Self {
        let description = Self::get_description(&technique);
        let steps = Self::get_steps(&technique);
        let related_smells = Self::get_related_smells(&smell_type);

        Self {
            smell_type,
            technique,
            priority,
            description,
            steps,
            related_smells,
        }
    }

    /// Create from a detected code smell
    pub fn from_code_smell(smell: &CodeSmell) -> Option<Self> {
        let techniques = smell.smell_type.recommended_refactorings();
        if techniques.is_empty() {
            return None;
        }

        // Use the first (primary) recommendation
        let technique = techniques[0].clone();
        let priority = Self::severity_to_priority(&smell.severity);

        Some(Self::new(smell.smell_type.clone(), technique, priority))
    }

    fn severity_to_priority(severity: &Severity) -> Priority {
        match severity {
            Severity::Critical => Priority::Critical,
            Severity::Error => Priority::High,
            Severity::Warning => Priority::Medium,
            Severity::Info => Priority::Low,
        }
    }

    fn get_description(technique: &RefactoringTechnique) -> String {
        match technique {
            RefactoringTechnique::ExtractMethod => {
                "Turn a code fragment into a separate method to improve readability and reusability"
                    .to_string()
            }
            RefactoringTechnique::MoveMethod => {
                "Move a method to the class where it's most used to improve cohesion".to_string()
            }
            RefactoringTechnique::ExtractClass => {
                "Create a new class and move fields and methods to it to improve separation of concerns"
                    .to_string()
            }
            RefactoringTechnique::IntroduceParameterObject => {
                "Replace a group of parameters that often appear together with a single object"
                    .to_string()
            }
            RefactoringTechnique::ReplaceConditionalWithPolymorphism => {
                "Move conditional logic to subclasses to use polymorphism instead of conditionals"
                    .to_string()
            }
            RefactoringTechnique::HideDelegate => {
                "Create delegate methods on the server to hide the delegate from the client".to_string()
            }
            RefactoringTechnique::RemoveMiddleMan => {
                "Remove delegation methods and let clients call the delegate directly".to_string()
            }
            _ => format!("Apply {} refactoring technique", technique),
        }
    }

    fn get_steps(technique: &RefactoringTechnique) -> Vec<String> {
        match technique {
            RefactoringTechnique::ExtractMethod => vec![
                "Create a new method with a name that clearly describes its purpose".to_string(),
                "Copy the relevant code fragment to the new method".to_string(),
                "Replace the original code with a call to the new method".to_string(),
                "Compile and test to ensure behavior is preserved".to_string(),
            ],
            RefactoringTechnique::MoveMethod => vec![
                "Identify the target class where the method belongs".to_string(),
                "Create a copy of the method in the target class".to_string(),
                "Update references to use the new location".to_string(),
                "Remove the method from the original class".to_string(),
                "Test to ensure behavior is preserved".to_string(),
            ],
            RefactoringTechnique::ExtractClass => vec![
                "Create a new class with a clear purpose".to_string(),
                "Use Move Field and Move Method to transfer relevant members".to_string(),
                "Update clients to use the new class".to_string(),
                "Remove transferred members from the original class".to_string(),
                "Test to ensure behavior is preserved".to_string(),
            ],
            RefactoringTechnique::IntroduceParameterObject => vec![
                "Create a new class or struct to hold the related parameters".to_string(),
                "Add the parameters as fields to the new class".to_string(),
                "Replace parameter groups with the new object in method signatures".to_string(),
                "Update call sites to pass the new object".to_string(),
                "Test to ensure behavior is preserved".to_string(),
            ],
            RefactoringTechnique::ReplaceConditionalWithPolymorphism => vec![
                "Identify the condition that varies by type".to_string(),
                "Create or identify subclasses for each variant".to_string(),
                "Move conditional branches to subclass methods".to_string(),
                "Replace conditional with polymorphic method call".to_string(),
                "Test to ensure behavior is preserved".to_string(),
            ],
            _ => vec![
                format!("1. Understand the current structure"),
                format!("2. Apply the {} transformation", technique),
                "3. Update all references".to_string(),
                "4. Run tests to verify behavior".to_string(),
            ],
        }
    }

    fn get_related_smells(smell_type: &SmellType) -> Vec<SmellType> {
        match smell_type {
            SmellType::LongFunction => vec![SmellType::DeepNesting, SmellType::TooManyReturns],
            SmellType::LargeClass => vec![SmellType::TooManyMethods, SmellType::TooManyFields],
            SmellType::LongParameterList => {
                vec![SmellType::DataClumps, SmellType::PrimitiveObsession]
            }
            SmellType::DuplicateCode => vec![],
            SmellType::FeatureEnvy => vec![SmellType::InappropriateIntimacy],
            SmellType::MessageChains => vec![SmellType::MiddleMan],
            _ => vec![],
        }
    }
}

impl SmellType {
    /// Get recommended refactoring techniques for this smell type
    pub fn recommended_refactorings(&self) -> Vec<RefactoringTechnique> {
        match self {
            // Bloaters
            Self::LongFunction => vec![
                RefactoringTechnique::ExtractMethod,
                RefactoringTechnique::ReplaceMethodWithMethodObject,
                RefactoringTechnique::DecomposeConditional,
            ],
            Self::LargeClass => vec![
                RefactoringTechnique::ExtractClass,
                RefactoringTechnique::ExtractSubclass,
                RefactoringTechnique::ExtractInterface,
            ],
            Self::PrimitiveObsession => vec![
                RefactoringTechnique::ReplaceArrayWithObject,
                RefactoringTechnique::ReplaceMagicLiteral,
            ],
            Self::LongParameterList => vec![
                RefactoringTechnique::IntroduceParameterObject,
                RefactoringTechnique::RemoveParameter,
                RefactoringTechnique::PreserveWholeObject,
            ],
            Self::DataClumps => vec![
                RefactoringTechnique::ExtractClass,
                RefactoringTechnique::IntroduceParameterObject,
            ],
            Self::SwitchStatements => vec![
                RefactoringTechnique::ReplaceConditionalWithPolymorphism,
                RefactoringTechnique::ExtractMethod,
                RefactoringTechnique::ReplaceTypeCodeWithStrategy,
            ],

            // OO Abusers
            Self::AlternativeClasses => vec![
                RefactoringTechnique::ExtractInterface,
                RefactoringTechnique::RenameMethod,
            ],
            Self::RefusedBequest => vec![
                RefactoringTechnique::ReplaceInheritanceWithDelegation,
                RefactoringTechnique::PushDownMethod,
                RefactoringTechnique::PushDownField,
            ],
            Self::TemporaryField => vec![
                RefactoringTechnique::ExtractClass,
                RefactoringTechnique::IntroduceNullObject,
            ],
            Self::DivergentChange => vec![
                RefactoringTechnique::ExtractClass,
                RefactoringTechnique::EncapsulateField,
            ],

            // Change Preventers
            Self::ParallelInheritance => vec![
                RefactoringTechnique::MoveMethod,
                RefactoringTechnique::MoveField,
            ],
            Self::ShotgunSurgery => vec![
                RefactoringTechnique::InlineMethod,
                RefactoringTechnique::InlineClass,
                RefactoringTechnique::MoveMethod,
            ],

            // Dispensables
            Self::Comments => vec![
                RefactoringTechnique::ExtractMethod,
                RefactoringTechnique::RenameMethod,
                RefactoringTechnique::IntroduceAssertion,
            ],
            Self::DuplicateCode => vec![
                RefactoringTechnique::ExtractMethod,
                RefactoringTechnique::PullUpMethod,
                RefactoringTechnique::FormTemplateMethod,
            ],
            Self::DataClass => vec![
                RefactoringTechnique::MoveMethod,
                RefactoringTechnique::EncapsulateField,
                RefactoringTechnique::RemoveSettingMethod,
            ],
            Self::DeadCode => vec![RefactoringTechnique::InlineMethod],
            Self::LazyClass => vec![
                RefactoringTechnique::InlineClass,
                RefactoringTechnique::CollapseHierarchy,
            ],
            Self::SpeculativeGenerality => vec![
                RefactoringTechnique::InlineClass,
                RefactoringTechnique::CollapseHierarchy,
                RefactoringTechnique::RemoveParameter,
            ],

            // Couplers
            Self::FeatureEnvy => vec![
                RefactoringTechnique::MoveMethod,
                RefactoringTechnique::ExtractMethod,
            ],
            Self::InappropriateIntimacy => vec![
                RefactoringTechnique::MoveMethod,
                RefactoringTechnique::MoveField,
                RefactoringTechnique::ExtractClass,
                RefactoringTechnique::HideDelegate,
            ],
            Self::MessageChains => vec![
                RefactoringTechnique::HideDelegate,
                RefactoringTechnique::ExtractMethod,
            ],
            Self::MiddleMan => vec![
                RefactoringTechnique::RemoveMiddleMan,
                RefactoringTechnique::InlineMethod,
                RefactoringTechnique::ReplaceDelegationWithInheritance,
            ],

            // Legacy compatibility
            Self::DeepNesting => vec![
                RefactoringTechnique::ExtractMethod,
                RefactoringTechnique::ReplaceNestedConditionalWithGuard,
                RefactoringTechnique::DecomposeConditional,
            ],
            Self::TooManyParameters => vec![
                RefactoringTechnique::IntroduceParameterObject,
                RefactoringTechnique::RemoveParameter,
            ],
            Self::TooManyMethods => vec![
                RefactoringTechnique::ExtractClass,
                RefactoringTechnique::ExtractInterface,
            ],
            Self::TooManyFields => vec![RefactoringTechnique::ExtractClass],
            Self::HighComplexity => vec![
                RefactoringTechnique::ExtractMethod,
                RefactoringTechnique::ReplaceConditionalWithPolymorphism,
                RefactoringTechnique::DecomposeConditional,
            ],
            Self::TooManyReturns => vec![
                RefactoringTechnique::ExtractMethod,
                RefactoringTechnique::ReplaceNestedConditionalWithGuard,
            ],
            Self::MagicNumber => vec![
                RefactoringTechnique::ReplaceMagicLiteral,
                RefactoringTechnique::ExtractVariable,
            ],
            Self::EmptyBlock => vec![
                RefactoringTechnique::ExtractMethod,
                RefactoringTechnique::IntroduceAssertion,
            ],
        }
    }
}

/// Placeholder - not a real refactoring technique
#[allow(non_upper_case_globals)]
pub const PreserveWholeObject: RefactoringTechnique =
    RefactoringTechnique::IntroduceParameterObject;

/// Refactoring recommendation engine
#[derive(Debug, Clone)]
pub struct RefactoringEngine {
    /// Minimum severity to generate recommendations for
    pub min_severity: Severity,
    /// Group recommendations by category
    pub group_by_category: bool,
}

impl Default for RefactoringEngine {
    fn default() -> Self {
        Self {
            min_severity: Severity::Warning,
            group_by_category: true,
        }
    }
}

impl RefactoringEngine {
    /// Create a new refactoring engine
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate recommendations from detected code smells
    pub fn generate_recommendations(&self, smells: &[CodeSmell]) -> Vec<RefactoringRecommendation> {
        smells
            .iter()
            .filter(|s| s.severity >= self.min_severity)
            .filter_map(RefactoringRecommendation::from_code_smell)
            .collect()
    }

    /// Group recommendations by category
    pub fn group_by_category<'a>(
        &self,
        recommendations: &'a [RefactoringRecommendation],
    ) -> std::collections::HashMap<SmellCategory, Vec<&'a RefactoringRecommendation>> {
        let mut grouped: std::collections::HashMap<
            SmellCategory,
            Vec<&'a RefactoringRecommendation>,
        > = std::collections::HashMap::new();

        for rec in recommendations {
            let category = rec.smell_type.category();
            grouped.entry(category).or_default().push(rec);
        }

        grouped
    }

    /// Get prioritized recommendations
    pub fn prioritize(
        &self,
        recommendations: Vec<RefactoringRecommendation>,
    ) -> Vec<RefactoringRecommendation> {
        let mut sorted = recommendations;
        sorted.sort_by(|a, b| b.priority.cmp(&a.priority));
        sorted
    }

    /// Get summary statistics
    pub fn summary(&self, recommendations: &[RefactoringRecommendation]) -> RefactoringSummary {
        let mut by_category = std::collections::HashMap::new();
        let mut by_priority = std::collections::HashMap::new();
        let mut by_technique = std::collections::HashMap::new();

        for rec in recommendations {
            *by_category.entry(rec.smell_type.category()).or_default() += 1;
            *by_priority.entry(rec.priority.clone()).or_default() += 1;
            *by_technique.entry(rec.technique.clone()).or_default() += 1;
        }

        RefactoringSummary {
            total: recommendations.len(),
            by_category,
            by_priority,
            by_technique,
        }
    }
}

/// Summary statistics for refactoring recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringSummary {
    /// Total number of recommendations
    pub total: usize,
    /// Count by smell category
    pub by_category: std::collections::HashMap<SmellCategory, usize>,
    /// Count by priority
    pub by_priority: std::collections::HashMap<Priority, usize>,
    /// Count by technique
    pub by_technique: std::collections::HashMap<RefactoringTechnique, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smell_refactorings_long_function() {
        let smell = SmellType::LongFunction;
        let refactorings = smell.recommended_refactorings();

        assert!(refactorings.contains(&RefactoringTechnique::ExtractMethod));
    }

    #[test]
    fn test_smell_refactorings_feature_envy() {
        let smell = SmellType::FeatureEnvy;
        let refactorings = smell.recommended_refactorings();

        assert!(refactorings.contains(&RefactoringTechnique::MoveMethod));
    }

    #[test]
    fn test_refactoring_recommendation_creation() {
        let rec = RefactoringRecommendation::new(
            SmellType::DuplicateCode,
            RefactoringTechnique::ExtractMethod,
            Priority::High,
        );

        assert_eq!(rec.smell_type, SmellType::DuplicateCode);
        assert_eq!(rec.technique, RefactoringTechnique::ExtractMethod);
        assert_eq!(rec.priority, Priority::High);
        assert!(!rec.steps.is_empty());
    }

    #[test]
    fn test_refactoring_from_code_smell() {
        let smell = CodeSmell {
            smell_type: SmellType::LongFunction,
            severity: Severity::Warning,
            file_path: "test.rs".to_string(),
            line_number: 10,
            symbol_name: "my_func".to_string(),
            message: "Function is too long".to_string(),
            metric_value: Some(100),
            threshold: Some(50),
            suggestion: Some("Break into smaller functions".to_string()),
        };

        let rec = RefactoringRecommendation::from_code_smell(&smell);

        assert!(rec.is_some());
        let rec = rec.unwrap();
        assert_eq!(rec.smell_type, SmellType::LongFunction);
        assert!(rec.technique == RefactoringTechnique::ExtractMethod);
        assert_eq!(rec.priority, Priority::Medium);
    }

    #[test]
    fn test_refactoring_engine() {
        let engine = RefactoringEngine::new();

        let smells = vec![
            CodeSmell {
                smell_type: SmellType::LongFunction,
                severity: Severity::Warning,
                file_path: "test.rs".to_string(),
                line_number: 10,
                symbol_name: "func1".to_string(),
                message: "Too long".to_string(),
                metric_value: None,
                threshold: None,
                suggestion: None,
            },
            CodeSmell {
                smell_type: SmellType::DuplicateCode,
                severity: Severity::Error,
                file_path: "test.rs".to_string(),
                line_number: 20,
                symbol_name: "func2".to_string(),
                message: "Duplicate code".to_string(),
                metric_value: None,
                threshold: None,
                suggestion: None,
            },
        ];

        let recommendations = engine.generate_recommendations(&smells);

        assert!(!recommendations.is_empty());
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Critical > Priority::High);
        assert!(Priority::High > Priority::Medium);
        assert!(Priority::Medium > Priority::Low);
    }

    #[test]
    fn test_technique_display() {
        assert_eq!(
            RefactoringTechnique::ExtractMethod.to_string(),
            "Extract Method"
        );
        assert_eq!(
            RefactoringTechnique::ReplaceConditionalWithPolymorphism.to_string(),
            "Replace Conditional with Polymorphism"
        );
    }
}
