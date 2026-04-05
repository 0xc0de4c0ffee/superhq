use gpui::Rgba;

pub struct FileIconInfo {
    pub icon: &'static str,
    pub color: Rgba,
}

// ── Icon paths ──────────────────────────────────────────────────────
const FOLDER: &str = "icons/files/folder.svg";
const FOLDER_OPEN: &str = "icons/files/folder-open.svg";
const FILE: &str = "icons/files/file.svg";
const FILE_CODE: &str = "icons/files/file-code.svg";
const FILE_TEXT: &str = "icons/files/file-text.svg";
const CHEVRON_RIGHT: &str = "icons/files/chevron-right.svg";
const CHEVRON_DOWN: &str = "icons/files/chevron-down.svg";

// ── Colors (muted for dark theme) ───────────────────────────────────
const FOLDER_CLR: Rgba = Rgba { r: 0.545, g: 0.671, b: 0.800, a: 0.80 };
const RUST_CLR: Rgba = Rgba { r: 0.871, g: 0.647, b: 0.518, a: 0.80 };
const BLUE_CLR: Rgba = Rgba { r: 0.361, g: 0.624, b: 0.847, a: 0.80 };
const YELLOW_CLR: Rgba = Rgba { r: 0.796, g: 0.796, b: 0.255, a: 0.80 };
const GO_CLR: Rgba = Rgba { r: 0.000, g: 0.678, b: 0.847, a: 0.80 };
const RUBY_CLR: Rgba = Rgba { r: 0.800, g: 0.204, b: 0.176, a: 0.80 };
const JAVA_CLR: Rgba = Rgba { r: 0.690, g: 0.447, b: 0.098, a: 0.80 };
const SWIFT_CLR: Rgba = Rgba { r: 0.941, g: 0.318, b: 0.220, a: 0.80 };
const STEEL_CLR: Rgba = Rgba { r: 0.400, g: 0.498, b: 0.616, a: 0.80 };
const YAML_CLR: Rgba = Rgba { r: 0.796, g: 0.380, b: 0.443, a: 0.80 };
const TOML_CLR: Rgba = Rgba { r: 0.612, g: 0.259, b: 0.129, a: 0.80 };
const XML_CLR: Rgba = Rgba { r: 0.890, g: 0.475, b: 0.200, a: 0.80 };
const HTML_CLR: Rgba = Rgba { r: 0.894, g: 0.302, b: 0.149, a: 0.80 };
const PURPLE_CLR: Rgba = Rgba { r: 0.627, g: 0.455, b: 0.769, a: 0.80 };
const SCSS_CLR: Rgba = Rgba { r: 0.804, g: 0.404, b: 0.600, a: 0.80 };
const MD_CLR: Rgba = Rgba { r: 0.318, g: 0.604, b: 0.729, a: 0.80 };
const GREEN_CLR: Rgba = Rgba { r: 0.537, g: 0.878, b: 0.318, a: 0.80 };
const SQL_CLR: Rgba = Rgba { r: 0.890, g: 0.549, b: 0.000, a: 0.80 };
const DOCKER_CLR: Rgba = Rgba { r: 0.271, g: 0.557, b: 0.902, a: 0.80 };
const GIT_CLR: Rgba = Rgba { r: 0.831, g: 0.341, b: 0.231, a: 0.80 };
const LOCK_CLR: Rgba = Rgba { r: 0.400, g: 0.400, b: 0.400, a: 0.53 };
const SVG_CLR: Rgba = Rgba { r: 0.890, g: 0.671, b: 0.200, a: 0.80 };
const ENV_CLR: Rgba = Rgba { r: 0.796, g: 0.796, b: 0.255, a: 0.60 };
const MAKE_CLR: Rgba = Rgba { r: 0.890, g: 0.475, b: 0.200, a: 0.80 };
const PRETTIER_CLR: Rgba = Rgba { r: 0.337, g: 0.702, b: 0.706, a: 0.80 };
const BIOME_CLR: Rgba = Rgba { r: 0.376, g: 0.647, b: 0.980, a: 0.80 };
const BUNDLER_CLR: Rgba = Rgba { r: 1.000, g: 0.753, b: 0.094, a: 0.80 };
const NGINX_CLR: Rgba = Rgba { r: 0.000, g: 0.588, b: 0.224, a: 0.80 };
const RENDER_CLR: Rgba = Rgba { r: 0.275, g: 0.890, b: 0.718, a: 0.80 };
const GRAPHQL_CLR: Rgba = Rgba { r: 0.898, g: 0.208, b: 0.671, a: 0.80 };
const RED_CLR: Rgba = Rgba { r: 0.878, g: 0.333, b: 0.333, a: 0.80 };
const CONFIG_CLR: Rgba = Rgba { r: 1.0, g: 1.0, b: 1.0, a: 0.40 };
const TEXT_CLR: Rgba = Rgba { r: 1.0, g: 1.0, b: 1.0, a: 0.67 };
const DEFAULT_CLR: Rgba = Rgba { r: 1.0, g: 1.0, b: 1.0, a: 0.33 };

pub fn folder_icon(is_open: bool) -> FileIconInfo {
    FileIconInfo {
        icon: if is_open { FOLDER_OPEN } else { FOLDER },
        color: FOLDER_CLR,
    }
}

pub fn chevron_icon(is_expanded: bool) -> &'static str {
    if is_expanded { CHEVRON_DOWN } else { CHEVRON_RIGHT }
}

pub fn file_icon(name: &str) -> FileIconInfo {
    let lower = name.to_lowercase();

    if let Some(info) = match_filename(&lower) {
        return info;
    }

    let ext = lower.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => FileIconInfo { icon: FILE_CODE, color: RUST_CLR },
        "ts" | "tsx" | "mts" | "cts" => FileIconInfo { icon: FILE_CODE, color: BLUE_CLR },
        "js" | "jsx" | "mjs" | "cjs" => FileIconInfo { icon: FILE_CODE, color: YELLOW_CLR },
        "py" | "pyi" | "pyw" => FileIconInfo { icon: FILE_CODE, color: BLUE_CLR },
        "go" => FileIconInfo { icon: FILE_CODE, color: GO_CLR },
        "rb" | "erb" | "gemspec" => FileIconInfo { icon: FILE_CODE, color: RUBY_CLR },
        "java" | "kt" | "kts" => FileIconInfo { icon: FILE_CODE, color: JAVA_CLR },
        "swift" => FileIconInfo { icon: FILE_CODE, color: SWIFT_CLR },
        "c" | "cpp" | "cc" | "cxx" | "h" | "hpp" | "cs" => FileIconInfo { icon: FILE_CODE, color: STEEL_CLR },
        "sh" | "bash" | "zsh" | "fish" => FileIconInfo { icon: FILE_CODE, color: GREEN_CLR },
        "html" | "htm" => FileIconInfo { icon: FILE_CODE, color: HTML_CLR },
        "css" => FileIconInfo { icon: FILE_CODE, color: PURPLE_CLR },
        "scss" | "sass" | "less" => FileIconInfo { icon: FILE_CODE, color: SCSS_CLR },
        "vue" | "svelte" => FileIconInfo { icon: FILE_CODE, color: GO_CLR },
        "json" | "jsonc" | "json5" => FileIconInfo { icon: FILE, color: YELLOW_CLR },
        "yaml" | "yml" => FileIconInfo { icon: FILE, color: YAML_CLR },
        "toml" => FileIconInfo { icon: FILE, color: TOML_CLR },
        "xml" | "xsl" | "xslt" => FileIconInfo { icon: FILE, color: XML_CLR },
        "ini" | "cfg" | "conf" | "config" => FileIconInfo { icon: FILE, color: CONFIG_CLR },
        "env" => FileIconInfo { icon: FILE, color: ENV_CLR },
        "sql" => FileIconInfo { icon: FILE, color: SQL_CLR },
        "csv" | "tsv" => FileIconInfo { icon: FILE, color: GREEN_CLR },
        "md" | "mdx" | "markdown" => FileIconInfo { icon: FILE_TEXT, color: MD_CLR },
        "txt" | "text" => FileIconInfo { icon: FILE_TEXT, color: TEXT_CLR },
        "pdf" => FileIconInfo { icon: FILE_TEXT, color: RED_CLR },
        "rst" => FileIconInfo { icon: FILE_TEXT, color: MD_CLR },
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico" | "tiff" => {
            FileIconInfo { icon: FILE, color: PURPLE_CLR }
        }
        "svg" => FileIconInfo { icon: FILE, color: SVG_CLR },
        "lock" => FileIconInfo { icon: FILE, color: LOCK_CLR },
        "wasm" => FileIconInfo { icon: FILE_CODE, color: PURPLE_CLR },
        "graphql" | "gql" => FileIconInfo { icon: FILE_CODE, color: GRAPHQL_CLR },
        "proto" => FileIconInfo { icon: FILE_CODE, color: TEXT_CLR },
        _ => FileIconInfo { icon: FILE, color: DEFAULT_CLR },
    }
}

fn match_filename(name: &str) -> Option<FileIconInfo> {
    Some(match name {
        "dockerfile" | "containerfile" => FileIconInfo { icon: FILE, color: DOCKER_CLR },
        "docker-compose.yml" | "docker-compose.yaml" | "compose.yml" | "compose.yaml" => {
            FileIconInfo { icon: FILE, color: DOCKER_CLR }
        }
        ".gitignore" | ".gitattributes" | ".gitmodules" => FileIconInfo { icon: FILE, color: GIT_CLR },
        "cargo.toml" | "cargo.lock" => FileIconInfo { icon: FILE, color: RUST_CLR },
        "package.json" | "package-lock.json" => FileIconInfo { icon: FILE, color: YELLOW_CLR },
        "tsconfig.json" | "tsconfig.node.json" => FileIconInfo { icon: FILE, color: BLUE_CLR },
        "makefile" | "justfile" | "rakefile" => FileIconInfo { icon: FILE, color: MAKE_CLR },
        "license" | "licence" | "license.md" | "licence.md" => {
            FileIconInfo { icon: FILE_TEXT, color: YELLOW_CLR }
        }
        "readme.md" | "readme" | "readme.txt" => FileIconInfo { icon: FILE_TEXT, color: MD_CLR },
        ".env" | ".env.local" | ".env.development" | ".env.production" => {
            FileIconInfo { icon: FILE, color: ENV_CLR }
        }
        "bun.lock" | "bun.lockb" | "yarn.lock" | "pnpm-lock.yaml" => {
            FileIconInfo { icon: FILE, color: LOCK_CLR }
        }
        ".eslintrc" | ".eslintrc.json" | ".eslintrc.js" | "eslint.config.js" | "eslint.config.mjs" => {
            FileIconInfo { icon: FILE, color: PURPLE_CLR }
        }
        ".prettierrc" | ".prettierrc.json" | "prettier.config.js" => {
            FileIconInfo { icon: FILE, color: PRETTIER_CLR }
        }
        "biome.json" | "biome.jsonc" => FileIconInfo { icon: FILE, color: BIOME_CLR },
        "vite.config.ts" | "vite.config.js" | "webpack.config.js" | "rollup.config.js" => {
            FileIconInfo { icon: FILE, color: BUNDLER_CLR }
        }
        "nginx.conf" => FileIconInfo { icon: FILE, color: NGINX_CLR },
        "render.yaml" | "render.yml" => FileIconInfo { icon: FILE, color: RENDER_CLR },
        _ => return None,
    })
}
