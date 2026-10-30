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
//! So far it covers tmpl's subset A, one module per library; anything
//! else is [`NotYet`]. No source maps yet.

pub mod connected;
pub mod names;
pub mod ops;
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

/// Translates a library, laid out as Kotlin's Java backend lays it out:
///
/// ```text
/// hello-world/pom.xml                                        Maven's, depending on temper-core
/// hello-world/src/main/java/hello_world/HelloWorldGlobal.java  the library's root module
/// hello-world/src/main/java/hello_world/HelloWorldMain.java    its entry point
/// ```
///
/// The package takes the library's name, with `_` for `-`
/// ([`names::package`]); the classes take it in Pascal case. Kotlin also
/// copies `temper-core` beside the package; this doesn't, yet.
pub fn translate(p: &Program, variant: &Variant) -> Result<Vec<OutFile>, NotYet> {
    let [module] = p.modules.as_slice() else {
        return Err(NotYet {
            what: format!("a library of {} modules", p.modules.len()),
        });
    };
    if !module.path.is_empty() {
        return Err(NotYet {
            what: "a module that isn't at the library's root".into(),
        });
    }
    let names = assign(p, &variant.rules);
    if let Some(clash) = names.clashes().first() {
        return Err(NotYet {
            what: format!("renaming an export: {clash}"),
        });
    }
    let library = &p.library.name;
    let package = names::package(library);
    let base = names::class_base(&package);
    let global = format!("{base}Global");
    let main = format!("{base}Main");
    let mut translator = translate::Translator::new(p, &names, &package, &global);
    let class = translator.module(module)?;
    let dir = format!("{library}/src/main/java/{package}");
    Ok(vec![
        OutFile {
            path: format!("{library}/pom.xml"),
            text: pom(library, &package, &p.library.version, &main, variant),
        },
        OutFile {
            path: format!("{dir}/{global}.java"),
            text: tree::class(&class),
        },
        OutFile {
            path: format!("{dir}/{main}.java"),
            text: main_class(&package, &main, &global),
        },
    ])
}

/// The entry point, as Kotlin's: logging to stdout, then the module's
/// class loaded, which runs its static initializer, then any async tasks
/// waited for.
fn main_class(package: &str, main: &str, global: &str) -> String {
    format!(
        "package {package};

import temper.core.Core;

public final class {main} {{
    private {main}() {{}}

    public static void main(String[] args) throws ClassNotFoundException {{
        Core.initSimpleLogging();
        Class.forName(\"{package}.{global}\");
        Core.waitUntilTasksComplete();
    }}
}}
"
    )
}

/// The package description, as Kotlin's `pomXml` writes it for a library
/// without tests: Maven Central's metadata and plugins, the Java release,
/// and the main class for the jar's manifest and `exec:java`.
fn pom(library: &str, package: &str, version: &str, main: &str, variant: &Variant) -> String {
    let main = format!("{package}.{main}");
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
    </dependency>
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
        <executions>
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
          </execution>
        </executions>
      </plugin>
    </plugins>
  </build>
</project>
"#
    )
}
