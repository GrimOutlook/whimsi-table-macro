use crate::msi_tables;
use pretty_assertions::assert_eq;
use quote::ToTokens;
use quote::quote;

#[test]
fn test_msi_table_with_generated_identifier() {
    let input = quote! {
        #[msi_table(name = "Directory")]
        struct Directory {
            #[msi_column(primary_key, identifier(generated), category = msi::Category::Identifier, length = 72)]
            directory: DirectoryIdentifier,
            #[msi_column(identifier(foreign_key = "Directory"), column_name = "Directory_Parent", category = msi::Category::Identifier, length = 72)]
            parent_directory: Option<DirectoryIdentifier>,
            #[msi_column(localizable, category = msi::Category::DefaultDir, length = 255)]
            default_dir: DefaultDir,
        }
    };

    // Call the macro's internal function
    let output = msi_tables::gen_tables_impl(input);

    let expected_output = quote! {
        use whimsi_lib::types::column::identifier::Identifier;
        use whimsi_lib::types::column::identifier::ToIdentifier;
        use whimsi_lib::types::helpers::id_generator::IdentifierGenerator;

        #[doc = "This is a simple wrapper around `Identifier` for the `DirectoryTable`. Used to ensure that identifiers for the `DirectoryTable` are only used in valid locations."]
        pub struct DirectoryIdentifier(Identifier);

        impl ToIdentifier for DirectoryIdentifier {
            fn to_identifier(&self) -> Identifier {
                self.0
            }
        }

        impl std::str::FromStr for DirectoryIdentifier {
            type Err = anyhow::Error;

            fn from_str(s: &str) -> anyhow::Result<Self> {
                Ok(Self(Identifier::from_str(s)?))
            }
        }

        #[derive(Debug, Clone, Default, PartialEq)]
        pub(crate) struct DirectoryIdentifierGenerator {
            count: usize,
            // A reference to a vec of all used Identifiers that should not be generated again.
            // These are all identifiers that inhabit a primary_key column.
            used: std::rc::Rc<std::cell::RefCell<Vec<Identifier>>>,
        }

        impl IdentifierGenerator for DirectoryIdentifierGenerator {
            type IdentifierType = DirectoryIdentifier;

            fn id_prefix(&self) -> &str {
                "DIRECTORY"
            }

            fn used(&self) -> &std::rc::Rc<std::cell::RefCell<Vec<Identifier>>> {
                &self.used
            }

            fn count(&self) -> usize {
                self.count
            }

            fn count_mut(&mut self) -> &mut usize {
                &mut self.count
            }
        }

        impl From<std::rc::Rc<std::cell::RefCell<Vec<Identifier>>>> for DirectoryIdentifierGenerator {
            fn from(value: std::rc::Rc<std::cell::RefCell<Vec<Identifier>>>) -> Self {
                let count = value.borrow().len();
                Self {
                    used: value,
                    count: 0,
                }
            }
        }

        pub struct DirectoryDao {
            directory: DirectoryIdentifier,
            parent_directory: Option<DirectoryIdentifier>,
            default_dir: DefaultDir,
        }

        impl PrimaryIdentifier for DirectoryDao {
            fn primary_identifier(&self) -> Option<Identifier> {
                Some( self.directory.to_identifier() )
            }
        }

        impl MsiDao for DirectoryDao {

            fn conflicts_with(&self, other: &Self) -> bool {
                self.directory == other.directory
            }

            fn to_row(&self) -> Vec<msi::Value> {
                vec![
                    msi::ToValue::to_value(self.directory),
                    msi::ToValue::to_value(self.parent_directory),
                    msi::ToValue::to_value(self.default_dir),
                ]
            }
        }

        pub struct DirectoryTable {
            generator: DirectoryIdentifierGenerator,
            entries: Vec<DirectoryDao>,
        }

        impl MsiTable for DirectoryTable {
            type TableValue = DirectoryDao;

            fn name(&self) -> &'static str {
                "Directory"
            }

            fn entries(&self) -> &Vec<DirectoryDao> {
                &self.entries
            }

            fn entries_mut(&mut self) -> &mut Vec<DirectoryDao> {
                &mut self.entries
            }

            fn primary_key_indices(&self) -> Vec<usize> {
                vec![0usize,]
            }

            fn primary_keys(&self) -> Vec<msi::ColumnType> {
                vec![self.directory.into(),]
            }

            fn columns(&self) -> Vec<msi::Column> {
                vec![
                    msi::Column::build("Directory").primary_key().category(msi::Category::Identifier).string(72),
                    msi::Column::build("Directory_Parent").nullable().foreign_key("Directory", 0).category(msi::Category::Identifier).string(72),
                    msi::Column::build("DefaultDir").localizable().category(msi::Category::DefaultDir).string(255),
                ]
            }
        }

    };

    // Compare the generated output with the expected output (e.g., using syn and comparing ASTs)
    let parsed_output =
        syn::parse2::<syn::File>(output).expect("Failed to parse output of test data");
    let parsed_expected =
        syn::parse2::<syn::File>(expected_output).expect("Failed to parse reference test data");

    assert_eq!(
        parsed_output.to_token_stream().to_string(),
        parsed_expected.to_token_stream().to_string()
    );
}

#[test]
fn test_msi_table_without_generated_identifier() {
    let input = quote! {
        #[msi_table(name = "FeatureComponent")]
        struct FeatureComponentDao {
            #[msi_column(primary_key, identifier(foreign_key = "Feature"), category = msi::Category::Identifier, length = 72)]
            feature_: FeatureIdentifier,
            #[msi_column(primary_key, identifier(foreign_key = "Component"), category = msi::Category::Identifier, length = 72)]
            component_: ComponentIdentifier,
        }
    };

    // Call the macro's internal function
    let output = msi_tables::gen_tables_impl(input);

    let expected_output = quote! {
        use whimsi_lib::types::column::identifier::Identifier;
        use whimsi_lib::types::column::identifier::ToIdentifier;
        use whimsi_lib::types::helpers::id_generator::IdentifierGenerator;

        pub struct FeatureComponentDao {
            feature_: FeatureIdentifier,
            component_: ComponentIdentifier,
        }

        impl PrimaryIdentifier for FeatureComponentDao {
            fn primary_identifier(&self) -> Option<Identifier> {
                None
            }
        }

        impl MsiDao for FeatureComponentDao {

            fn conflicts_with(&self, other: &Self) -> bool {
                self.feature_ == other.feature_ && self.component_ == other.component_
            }

            fn to_row(&self) -> Vec<msi::Value> {
                vec![
                    msi::ToValue::to_value(self.feature_),
                    msi::ToValue::to_value(self.component_),
                ]
            }
        }

        pub struct FeatureComponentTable {
            entries: Vec<FeatureComponentDao>,
        }

        impl MsiTable for FeatureComponentTable {
            type TableValue = FeatureComponentDao;

            fn name(&self) -> &'static str {
                "FeatureComponent"
            }

            fn entries(&self) -> &Vec<FeatureComponentDao> {
                &self.entries
            }

            fn entries_mut(&mut self) -> &mut Vec<FeatureComponentDao> {
                &mut self.entries
            }

            fn primary_key_indices(&self) -> Vec<usize> {
                vec![0usize,1usize,]
            }

            fn primary_keys(&self) -> Vec<msi::ColumnType> {
                vec![self.feature_.into(), self.component_.into(),]
            }

            fn columns(&self) -> Vec<msi::Column> {
                vec![
                    msi::Column::build("Feature_").primary_key().foreign_key("Feature", 0).category(msi::Category::Identifier).string(72),
                    msi::Column::build("Component_").primary_key().foreign_key("Component", 0).category(msi::Category::Identifier).string(72),
                ]
            }
        }

    };

    // Compare the generated output with the expected output (e.g., using syn and comparing ASTs)
    let parsed_output =
        syn::parse2::<syn::File>(output).expect("Failed to parse output of test data");
    let parsed_expected =
        syn::parse2::<syn::File>(expected_output).expect("Failed to parse reference test data");

    assert_eq!(
        parsed_output.to_token_stream().to_string(),
        parsed_expected.to_token_stream().to_string()
    );
}

#[test]
fn test_msi_tables_enum() {
    let input = quote! {
        enum MsiTables {
            Directory {
                #[msi_column(primary_key, identifier(generated), category = msi::Category::Identifier, length = 72)]
                directory: DirectoryIdentifier,
                #[msi_column(identifier(foreign_key = "Directory"), column_name = "Directory_Parent", category = msi::Category::Identifier, length = 72)]
                parent_directory: Option<DirectoryIdentifier>,
                #[msi_column(localizable, category = msi::Category::DefaultDir, length = 255)]
                default_dir: DefaultDir,
            },

            FeatureComponent {
                #[msi_column(primary_key, identifier(foreign_key = "Feature"), category = msi::Category::Identifier, length = 72)]
                feature_: FeatureIdentifier,
                #[msi_column(primary_key, identifier(foreign_key = "Component"), category = msi::Category::Identifier, length = 72)]
                component_: ComponentIdentifier,
            }
        }
    };

    // Call the macro's internal function
    let output = msi_tables::gen_tables_impl(input);

    let expected_output = quote! {
        use whimsi_lib::types::column::identifier::Identifier;
        use whimsi_lib::types::column::identifier::ToIdentifier;
        use whimsi_lib::types::helpers::id_generator::IdentifierGenerator;

        pub enum MsiTables {
            Directory(DirectoryTable),
            FeatureComponent(FeatureComponentTable),
        }

        #[doc = "This is a simple wrapper around `Identifier` for the `DirectoryTable`. Used to ensure that identifiers for the `DirectoryTable` are only used in valid locations."]
        pub struct DirectoryIdentifier(Identifier);

        impl ToIdentifier for DirectoryIdentifier {
            fn to_identifier(&self) -> Identifier {
                self.0
            }
        }
        impl std::str::FromStr for DirectoryIdentifier {
            type Err = anyhow::Error;

            fn from_str(s: &str) -> anyhow::Result<Self> {
                Ok(Self(Identifier::from_str(s)?))
            }
        }

        #[derive(Debug, Clone, Default, PartialEq)]
        pub(crate) struct DirectoryIdentifierGenerator {
            count: usize,
            // A reference to a vec of all used Identifiers that should not be generated again.
            // These are all identifiers that inhabit a primary_key column.
            used: std::rc::Rc<std::cell::RefCell<Vec<Identifier>>>,
        }

        impl IdentifierGenerator for DirectoryIdentifierGenerator {
            type IdentifierType = DirectoryIdentifier;

            fn id_prefix(&self) -> &str {
                "DIRECTORY"
            }

            fn used(&self) -> &std::rc::Rc<std::cell::RefCell<Vec<Identifier>>> {
                &self.used
            }

            fn count(&self) -> usize {
                self.count
            }

            fn count_mut(&mut self) -> &mut usize {
                &mut self.count
            }
        }

        impl From<std::rc::Rc<std::cell::RefCell<Vec<Identifier>>>> for DirectoryIdentifierGenerator {
            fn from(value: std::rc::Rc<std::cell::RefCell<Vec<Identifier>>>) -> Self {
                let count = value.borrow().len();
                Self {
                    used: value,
                    count: 0,
                }
            }
        }

        pub struct DirectoryDao {
            directory: DirectoryIdentifier,
            parent_directory: Option<DirectoryIdentifier>,
            default_dir: DefaultDir,
        }

        impl PrimaryIdentifier for DirectoryDao {
            fn primary_identifier(&self) -> Option<Identifier> {
                Some( self.directory.to_identifier() )
            }
        }

        impl MsiDao for DirectoryDao {

            fn conflicts_with(&self, other: &Self) -> bool {
                self.directory == other.directory
            }

            fn to_row(&self) -> Vec<msi::Value> {
                vec![
                    msi::ToValue::to_value(self.directory),
                    msi::ToValue::to_value(self.parent_directory),
                    msi::ToValue::to_value(self.default_dir),
                ]
            }
        }

        pub struct DirectoryTable {
            generator: DirectoryIdentifierGenerator,
            entries: Vec<DirectoryDao>,
        }

        impl MsiTable for DirectoryTable {
            type TableValue = DirectoryDao;
            fn name(&self) -> &'static str {
                "Directory"
            }

            fn entries(&self) -> &Vec<DirectoryDao> {
                &self.entries
            }

            fn entries_mut(&mut self) -> &mut Vec<DirectoryDao> {
                &mut self.entries
            }
            fn primary_key_indices(&self) -> Vec<usize> {
                vec![0usize,]
            }

            fn primary_keys(&self) -> Vec<msi::ColumnType> {
                vec![self.directory.into(),]
            }

            fn columns(&self) -> Vec<msi::Column> {
                vec![
                    msi::Column::build("Directory").primary_key().category(msi::Category::Identifier).string(72),
                    msi::Column::build("Directory_Parent").nullable().foreign_key("Directory", 0).category(msi::Category::Identifier).string(72),
                    msi::Column::build("DefaultDir").localizable().category(msi::Category::DefaultDir).string(255),
                ]
            }
        }

        pub struct FeatureComponentDao {
            feature_: FeatureIdentifier,
            component_: ComponentIdentifier,
        }

        impl PrimaryIdentifier for FeatureComponentDao {
            fn primary_identifier(&self) -> Option<Identifier> {
                None
            }
        }

        impl MsiDao for FeatureComponentDao {

            fn conflicts_with(&self, other: &Self) -> bool {
                self.feature_ == other.feature_ && self.component_ == other.component_
            }

            fn to_row(&self) -> Vec<msi::Value> {
                vec![
                    msi::ToValue::to_value(self.feature_),
                    msi::ToValue::to_value(self.component_),
                ]
            }
        }

        pub struct FeatureComponentTable {
            entries: Vec<FeatureComponentDao>,
        }

        impl MsiTable for FeatureComponentTable {
            type TableValue = FeatureComponentDao;

            fn name(&self) -> &'static str {
                "FeatureComponent"
            }

            fn entries(&self) -> &Vec<FeatureComponentDao> {
                &self.entries
            }

            fn entries_mut(&mut self) -> &mut Vec<FeatureComponentDao> {
                &mut self.entries
            }

            fn primary_key_indices(&self) -> Vec<usize> {
                vec![0usize,1usize,]
            }

            fn primary_keys(&self) -> Vec<msi::ColumnType> {
                vec![self.feature_.into(),self.component_.into(),]
            }

            fn columns(&self) -> Vec<msi::Column> {
                vec![
                    msi::Column::build("Feature_").primary_key().foreign_key("Feature", 0).category(msi::Category::Identifier).string(72),
                    msi::Column::build("Component_").primary_key().foreign_key("Component", 0).category(msi::Category::Identifier).string(72),
                ]
            }
        }
    };

    // Compare the generated output with the expected output (e.g., using syn and comparing ASTs)
    let parsed_output = syn::parse2::<syn::File>(output.clone())
        .unwrap_or_else(|_| panic!("Failed to parse output of test data:\n{}", output));
    let parsed_expected =
        syn::parse2::<syn::File>(expected_output).expect("Failed to parse reference test data");

    assert_eq!(
        parsed_output.to_token_stream().to_string(),
        parsed_expected.to_token_stream().to_string()
    );
}
