#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceLanguage {
    Rust,
    Python,
    Go,
    TypeScript,
    JavaScript,
    C,
    Cpp,
    Java,
    Php,
    Ruby,
    Kotlin,
    Swift,
}

impl SourceLanguage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::Go => "go",
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Java => "java",
            Self::Php => "php",
            Self::Ruby => "ruby",
            Self::Kotlin => "kotlin",
            Self::Swift => "swift",
        }
    }

    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Rust => &["rs"],
            Self::Python => &["py"],
            Self::Go => &["go"],
            Self::TypeScript => &["ts", "tsx"],
            Self::JavaScript => &["js", "jsx", "mjs", "cjs"],
            Self::C => &["c", "h"],
            Self::Cpp => &["cc", "cpp", "cxx", "hpp", "hh", "h"],
            Self::Java => &["java"],
            Self::Php => &["php"],
            Self::Ruby => &["rb"],
            Self::Kotlin => &["kt", "kts"],
            Self::Swift => &["swift"],
        }
    }

    pub fn primary_extension(self) -> &'static str {
        self.extensions()[0]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepoFixture {
    pub language: SourceLanguage,
    pub slug: &'static str,
    pub clone_url: &'static str,
    pub commit_sha: &'static str,
    pub entry_symbol: &'static str,
}

pub const PROJECT_FIXTURES: [RepoFixture; 12] = [
    RepoFixture {
        language: SourceLanguage::Rust,
        slug: "serde-rs/serde",
        clone_url: "https://github.com/serde-rs/serde.git",
        commit_sha: "fa7da4a93567ed347ad0735c28e439fca688ef26",
        entry_symbol: "Serialize",
    },
    RepoFixture {
        language: SourceLanguage::Python,
        slug: "pallets/click",
        clone_url: "https://github.com/pallets/click.git",
        commit_sha: "cdab890e57a30a9f437b88ce9652f7bfce980c1f",
        entry_symbol: "Command",
    },
    RepoFixture {
        language: SourceLanguage::Go,
        slug: "spf13/cobra",
        clone_url: "https://github.com/spf13/cobra.git",
        commit_sha: "61968e893eee2f27696c2fbc8e34fa5c4afaf7c4",
        entry_symbol: "Execute",
    },
    RepoFixture {
        language: SourceLanguage::TypeScript,
        slug: "axios/axios",
        clone_url: "https://github.com/axios/axios.git",
        commit_sha: "ebc6056adc341b1bcc7c940262391c2b4c7223b6",
        entry_symbol: "Axios",
    },
    RepoFixture {
        language: SourceLanguage::JavaScript,
        slug: "expressjs/express",
        clone_url: "https://github.com/expressjs/express.git",
        commit_sha: "6c4249feec8ab40631817c8e7001baf2ed022224",
        entry_symbol: "express",
    },
    RepoFixture {
        language: SourceLanguage::C,
        slug: "libcheck/check",
        clone_url: "https://github.com/libcheck/check.git",
        commit_sha: "455005dc29dc6727de7ee36fee4b49a13b39f73f",
        entry_symbol: "main",
    },
    RepoFixture {
        language: SourceLanguage::Cpp,
        slug: "fmtlib/fmt",
        clone_url: "https://github.com/fmtlib/fmt.git",
        commit_sha: "ae6fd83e2ee09ac260f30bbd33f2071e99f972de",
        entry_symbol: "format",
    },
    RepoFixture {
        language: SourceLanguage::Java,
        slug: "google/gson",
        clone_url: "https://github.com/google/gson.git",
        commit_sha: "b7d59549188867deb42e46073fb38735a5beda1c",
        entry_symbol: "Gson",
    },
    RepoFixture {
        language: SourceLanguage::Php,
        slug: "Seldaek/monolog",
        clone_url: "https://github.com/Seldaek/monolog.git",
        commit_sha: "6db20ca029219dd8de378cea8e32ee149399ef1b",
        entry_symbol: "Logger",
    },
    RepoFixture {
        language: SourceLanguage::Ruby,
        slug: "sinatra/sinatra",
        clone_url: "https://github.com/sinatra/sinatra.git",
        commit_sha: "f891dd2b6f4911e356600efe6c3b82af97d262c6",
        entry_symbol: "Sinatra",
    },
    RepoFixture {
        language: SourceLanguage::Kotlin,
        slug: "InsertKoinIO/koin",
        clone_url: "https://github.com/InsertKoinIO/koin.git",
        commit_sha: "461b5684684bb1b17411f27c35a955cdc90f299b",
        entry_symbol: "startKoin",
    },
    RepoFixture {
        language: SourceLanguage::Swift,
        slug: "apple/swift-argument-parser",
        clone_url: "https://github.com/apple/swift-argument-parser.git",
        commit_sha: "1e77425a27b864b97501c78511382bd0a0500520",
        entry_symbol: "ArgumentParser",
    },
];
