//! Temper's Java backend: tmpl in, Java out.
//!
//! Works from tmpl alone (notes/tmpl/overview.md, "Boundaries"). Its
//! variants share the translation and differ only in data
//! (notes/tmpl/profiles.md, "Variants and the first profiles"); the first,
//! [`JAVA`], is Kotlin-compatible: the same Maven package and layout
//! (`pom.xml`, and a package per library, holding a module's statics in
//! `NAMEGlobal` and its entry point in `NAMEMain`), `temper-core` runtime,
//! public names, and Java 17, as Kotlin Temper's `java` backend, with
//! prettier bodies: no numbered names, no class names before calls and
//! fields of the class's own, `+` converting numbers in concatenations,
//! sorted imports, and labels only where a jump needs one. `javac
//! -Xlint:all -Werror` passes on the output.
//!
//! The pipeline: the naming pass ([`temper_tmpl::naming::assign`]) with
//! Java's rules ([`names::RULES`]), then [`translate`] into Java's output
//! tree ([`tree`]), with operators spelled by [`ops`] and core's connected
//! declarations by [`connected`], then the printer.
//!
//! So far it covers tmpl's subset A, with several modules per library,
//! imports between them and from other libraries, and tests; anything
//! else is [`NotYet`]. No source maps yet.

pub mod connected;
pub mod names;
pub mod ops;
pub mod runtime;
pub mod translate;
pub mod tree;

use std::fmt;
use temper_tmpl::naming::{NamingRules, assign};
use temper_tmpl::{Features, Profile, Program};

/// A named set of translation choices for Java.
pub struct Variant {
    /// Its name on the command line: `java`.
    pub name: &'static str,
    /// What it handles natively, which lowering and validation read.
    pub profile: Profile,
    pub rules: NamingRules,
    /// The `temper-core` version its output depends on.
    pub runtime_version: &'static str,
    /// The Java release its output targets, as `javac --release` and
    /// Maven's compiler plugin take it.
    pub java_release: &'static str,
}

/// The first variant: Kotlin-compatible API and names, pretty bodies,
/// Java 17. Everything is native except statements inside expressions,
/// which the hoisting pass will lower (not built yet: a program that needs
/// it is [`NotYet`]).
pub const JAVA: Variant = Variant {
    name: "java",
    profile: Profile {
        native: Features::NONE,
    },
    rules: names::RULES,
    runtime_version: "0.6.0",
    java_release: "17",
};

/// Every variant, by name.
pub const VARIANTS: &[&Variant] = &[&JAVA];

/// Something the translation doesn't cover yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotYet {
    pub what: String,
}

impl fmt::Display for NotYet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not yet in the Java backend: {}", self.what)
    }
}

impl std::error::Error for NotYet {}

/// A generated file: its path relative to the output directory, and its
/// text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutFile {
    pub path: String,
    pub text: String,
}

/// Translates a library, laid out as Kotlin's Java backend lays it out,
/// a package per module:
///
/// ```text
/// mods/pom.xml                                    Maven's, depending on temper-core
/// mods/src/main/java/mods/ModsGlobal.java         the library's root module
/// mods/src/main/java/mods/ModsMain.java           its entry point
/// mods/src/main/java/mods/helpers/HelpersGlobal.java  module `helpers`
/// mods/src/main/java/mods/helpers/HelpersMain.java    its entry point
/// mods/src/test/java/mods/ModsTest.java           the root module's tests
/// ```
///
/// The package takes the library's name, with `_` for `-`
/// ([`names::package`]), and a module's path after it; the classes take
/// the package's last part in Pascal case. Kotlin also copies
/// `temper-core` beside the package; `tempest build` does too
/// ([`runtime`]).
pub fn translate(p: &Program, variant: &Variant) -> Result<Vec<OutFile>, NotYet> {
    let names = assign(p, &variant.rules);
    if let Some(clash) = names.clashes().first() {
        return Err(NotYet {
            what: format!("renaming an export: {clash}"),
        });
    }
    let library = &p.library.name;
    let mut sources = Vec::new();
    let mut tests = Vec::new();
    let mut mains = Vec::new();
    let mut root_main = String::new();
    for module in &p.modules {
        let (package, base) = translate::module_names(p, module.decl);
        let global = format!("{base}Global");
        let main = format!("{base}Main");
        let mut translator = translate::Translator::new(p, &names, &package, &global);
        let class = translator.module(module)?;
        let dir = package.replace('.', "/");
        sources.push(OutFile {
            path: format!("{library}/src/main/java/{dir}/{global}.java"),
            text: tree::class(&class),
        });
        sources.push(OutFile {
            path: format!("{library}/src/main/java/{dir}/{main}.java"),
            text: main_class(&package, &main, &loads(p, module)),
        });
        if let Some(class) = translator.tests(module, &format!("{base}Test"))? {
            tests.push(format!("{package}.{base}Test"));
            sources.push(OutFile {
                path: format!("{library}/src/test/java/{dir}/{base}Test.java"),
                text: tree::test_class(&class),
            });
        }
        if module.path.is_empty() {
            root_main = format!("{package}.{main}");
        }
        mains.push(format!("{package}.{main}"));
    }
    let pom = pom(p, library, &root_main, &mains, &tests, variant);
    Ok([OutFile {
        path: format!("{library}/pom.xml"),
        text: pom,
    }]
    .into_iter()
    .chain(sources)
    .collect())
}

/// The classes a module's entry point loads, in order: those of other
/// libraries' modules it imports, then its own library's modules it
/// imports, directly or not, in the order they run, then its own. Each
/// class's static initializer is its module's init, which the JVM runs
/// when the class is first used, so this runs them in Temper's order,
/// imports first, even where nothing reads them.
fn loads(p: &Program, module: &temper_tmpl::Module) -> Vec<String> {
    use temper_tmpl::analysis::module_imports;
    let mut needed = vec![module.decl];
    let mut external = Vec::new();
    let mut i = 0;
    while i < needed.len() {
        let m = p.modules.iter().find(|m| m.decl == needed[i]);
        for import in m.map(|m| module_imports(p, m)).unwrap_or_default() {
            let own = p.modules.iter().any(|m| m.decl == import.from);
            if own && !needed.contains(&import.from) {
                needed.push(import.from);
            } else if !own && !external.contains(&import.from) {
                external.push(import.from);
            }
        }
        i += 1;
    }
    external.sort();
    let order = (p.modules.iter())
        .filter(|m| needed.contains(&m.decl))
        .map(|m| m.decl);
    let class = |m| {
        let (package, class) = translate::global_class(p, m);
        format!("{package}.{class}")
    };
    external.into_iter().chain(order).map(class).collect()
}

/// The entry point, as Kotlin's: logging to stdout, then the module's
/// class loaded, after those of the modules it imports
/// ([`loads`]), which runs their static initializers, then any async
/// tasks waited for.
fn main_class(package: &str, main: &str, loads: &[String]) -> String {
    let loads = (loads.iter())
        .map(|class| format!("        Class.forName(\"{class}\");\n"))
        .collect::<Vec<_>>()
        .concat();
    format!(
        "package {package};

import temper.core.Core;

public final class {main} {{
    private {main}() {{}}

    public static void main(String[] args) throws ClassNotFoundException {{
        Core.initSimpleLogging();
{loads}        Core.waitUntilTasksComplete();
    }}
}}
"
    )
}

/// The package description, as Kotlin's `pomXml` writes it for a library
/// without tests: Maven Central's metadata and plugins, the Java release,
/// and the main class for the jar's manifest and `exec:java`.
fn pom(
    p: &Program,
    library: &str,
    main: &str,
    mains: &[String],
    tests: &[String],
    variant: &Variant,
) -> String {
    let version = p.version_or_default();
    let package = names::package(library);
    // Each library used, by its group (its name as Java names a package)
    // and artifact (its name), as Kotlin's default configuration has them,
    // and std as Kotlin names it; for the tests alone, in their scope.
    let in_code = libraries_in_code(p);
    let mut dependencies = (p.dependencies().iter())
        .map(|&(name, version)| {
            let (group, artifact) = match name {
                "std" => ("dev.temperlang".to_string(), "temper-std"),
                _ => (name.replace('-', "_"), name),
            };
            let scope = if in_code.iter().any(|l| l == name) {
                "compile"
            } else {
                "test"
            };
            format!(
                r#"
    <dependency>
      <groupId>{group}</groupId>
      <artifactId>{artifact}</artifactId>
      <version>{}</version>
      <type>jar</type>
      <scope>{scope}</scope>
    </dependency>"#,
                temper_tmpl::version_or_default(version),
            )
        })
        .collect::<Vec<_>>()
        .concat();
    if !tests.is_empty() {
        dependencies.push_str(
            r#"
    <dependency>
      <groupId>org.junit.jupiter</groupId>
      <artifactId>junit-jupiter</artifactId>
      <version>5.9.2</version>
      <type>jar</type>
      <scope>test</scope>
    </dependency>"#,
        );
    }
    let executions = (mains.iter())
        .map(|main| {
            format!(
                r#"
          <execution>
            <id>{main}</id>
            <goals>
              <!-- Run in the Maven VM via `mvn compile exec:java@{main}` -->
              <goal>java</goal>
              <!-- Run in a forked JVM via `mvn compile exec:exec@{main}` -->
              <goal>exec</goal>
            </goals>
            <configuration>
              <!-- Used by exec:java -->
              <mainClass>{main}</mainClass>
              <!-- Used by exec:exec -->
              <executable>java</executable>
              <arguments>
                <argument>-classpath</argument>
                <argument>${{project.build.outputDirectory}}</argument>
                <argument>{main}</argument>
              </arguments>
            </configuration>
          </execution>"#
            )
        })
        .collect::<Vec<_>>()
        .concat();
    let surefire = if tests.is_empty() {
        String::new()
    } else {
        let includes = (tests.iter())
            .map(|t| format!("\n            <include>{t}</include>"))
            .collect::<Vec<_>>()
            .concat();
        format!(
            r#"
      <plugin>
        <groupId>org.apache.maven.plugins</groupId>
        <artifactId>maven-surefire-plugin</artifactId>
        <version>3.0.0</version>
        <configuration>
          <!-- Configuration is mostly documented in the source: https://github.com/apache/maven-surefire/blob/surefire-3.0.0/maven-surefire-plugin/src/main/java/org/apache/maven/plugin/surefire/SurefirePlugin.java -->
          <reportFormat>xml</reportFormat>
          <includes>{includes}
          </includes>
        </configuration>
      </plugin>"#
        )
    };
    let runtime = variant.runtime_version;
    let release = variant.java_release;
    format!(
        r#"<project xmlns="http://maven.apache.org/POM/4.0.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 https://maven.apache.org/xsd/maven-4.0.0.xsd">
  <modelVersion>4.0.0</modelVersion>
  <groupId>{package}</groupId>
  <artifactId>{library}</artifactId>
  <version>{version}</version>
  <name>{library}</name>
  <description>{library}</description>
  <properties>
    <project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>
    <project.reporting.outputEncoding>UTF-8</project.reporting.outputEncoding>
  </properties>
  <dependencies>
    <dependency>
      <groupId>dev.temperlang</groupId>
      <artifactId>temper-core</artifactId>
      <version>{runtime}</version>
      <type>jar</type>
      <scope>compile</scope>
    </dependency>{dependencies}
  </dependencies>
  <build>
    <plugins>
      <plugin>
        <groupId>org.sonatype.central</groupId>
        <artifactId>central-publishing-maven-plugin</artifactId>
        <version>0.8.0</version>
        <extensions>true</extensions>
        <configuration>
          <publishingServerId>central</publishingServerId>
          <autoPublish>true</autoPublish>
        </configuration>
      </plugin>
      <plugin>
        <groupId>org.apache.maven.plugins</groupId>
        <artifactId>maven-source-plugin</artifactId>
        <version>3.3.0</version>
        <executions>
          <execution>
            <id>attach-sources</id>
            <goals>
              <goal>jar-no-fork</goal>
            </goals>
          </execution>
        </executions>
      </plugin>
      <plugin>
        <groupId>org.apache.maven.plugins</groupId>
        <artifactId>maven-javadoc-plugin</artifactId>
        <version>3.5.0</version>
        <configuration>
          <failOnError>false</failOnError>
          <doclint>-missing</doclint>
          <docencoding>UTF-8</docencoding>
        </configuration>
        <executions>
          <execution>
            <id>attach-javadocs</id>
            <goals>
              <goal>jar</goal>
            </goals>
          </execution>
        </executions>
      </plugin>
      <plugin>
        <groupId>org.apache.maven.plugins</groupId>
        <artifactId>maven-gpg-plugin</artifactId>
        <version>3.1.0</version>
        <executions>
          <execution>
            <id>sign-artifacts</id>
            <phase>verify</phase>
            <goals>
              <goal>sign</goal>
            </goals>
            <configuration>
              <id>${{gpg.keyname}}</id>
              <id>${{gpg.keyname}}</id>
            </configuration>
          </execution>
        </executions>
      </plugin>
      <plugin>
        <groupId>org.apache.maven.plugins</groupId>
        <artifactId>maven-compiler-plugin</artifactId>
        <version>3.11.0</version>
        <configuration>
          <release>{release}</release>
        </configuration>
      </plugin>
      <plugin>
        <!-- https://maven.apache.org/plugins/maven-jar-plugin/ -->
        <groupId>org.apache.maven.plugins</groupId>
        <artifactId>maven-jar-plugin</artifactId>
        <version>3.3.0</version>
        <configuration>
          <archive>
            <!-- https://maven.apache.org/shared/maven-archiver/index.html details contents -->
            <manifest>
              <mainClass>{main}</mainClass>
            </manifest>
          </archive>
        </configuration>
      </plugin>
      <plugin>
        <!-- https://www.mojohaus.org/exec-maven-plugin/ -->
        <groupId>org.codehaus.mojo</groupId>
        <artifactId>exec-maven-plugin</artifactId>
        <version>3.1.0</version>
        <executions>{executions}
        </executions>
      </plugin>{surefire}
    </plugins>
  </build>
</project>
"#
    )
}

/// The libraries the library's own code needs, by name: all it uses,
/// but those only its tests use.
fn libraries_in_code(p: &Program) -> Vec<String> {
    let code = libraries_used(p, false);
    let tests = libraries_used(p, true);
    (p.dependencies().into_iter())
        .map(|(name, _)| name.to_string())
        .filter(|name| code.contains(name) || !tests.contains(name))
        .collect()
}

/// The libraries the library's own code uses, or with `tests`, its
/// tests, by name.
fn libraries_used(p: &Program, tests: bool) -> Vec<String> {
    use temper_tmpl::analysis::module_imports;
    use temper_tmpl::qname::library_of;
    let mut found = Vec::new();
    for module in &p.modules {
        let imports = if tests {
            temper_tmpl::analysis::test_imports(p, module)
        } else {
            module_imports(p, module)
        };
        for import in imports {
            for d in import.decls {
                if !temper_tmpl::qname::is_external(p, d) || library_of(p, d) == "core" {
                    continue;
                }
                let name = p.name(library_decl(p, d)).to_string();
                if !found.contains(&name) {
                    found.push(name);
                }
            }
        }
    }
    found
}

/// The `uses` library that owns an external declaration.
fn library_decl(p: &Program, decl: temper_tmpl::DeclId) -> temper_tmpl::DeclId {
    let mut d = decl;
    while p.decls.contains(d) && p.decls.get(d).kind != temper_tmpl::DeclKind::Library {
        d = p.decls.get(d).owner;
    }
    d
}
