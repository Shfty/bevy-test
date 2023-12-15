use std::borrow::Cow;

use bevy::{render::render_resource::Source, prelude::Shader};
use regex::Regex;

pub trait SourceEx {
    fn emplace(&mut self, from: &str, to: &str) -> &mut Self;
}

impl SourceEx for Source {
    fn emplace(&mut self, from: &str, to: &str) -> &mut Self {
        match self {
            Source::Wgsl(source) | Source::Glsl(source, _) => {
                let re = Regex::new(from).unwrap();
                if !re.is_match(source) {
                    panic!("Missing emplacement source for\n{to:}");
                }
                *source = re.replace(source, to).to_string().into();
            }
            Source::SpirV(_) => panic!("emplace_module does not support SPIR-V"),
        }

        self
    }
}

pub trait ShaderComposer {
    fn base(&self) -> Source;

    fn emplace(&self, source: &mut Source);

    fn compose(&self) -> Source {
        let mut source = self.base();

        self.emplace(&mut source);

        // Strip tags from non-overridden elements
        match &mut source {
            Source::Wgsl(source) | Source::Glsl(source, _) => {
                let re = Regex::new(r"module!\([\S]*\)").unwrap();
                let s = re.replace(source, "").to_string();

                let re = Regex::new(r"block!\([\S]*\)").unwrap();
                let s = re.replace(&s, "").to_string();

                let s = s.replace("#[function]\n", "");

                *source = Cow::Owned(s);
            }
            Source::SpirV(_) => panic!("compose does not support SPIR-V"),
        };

        source
    }

    fn as_wgsl(&self) -> String {
        match self.compose() {
            Source::Wgsl(wgsl) => wgsl.to_string(),
            _ => panic!("ShaderComposer source is not WGSL"),
        }
    }

    fn as_glsl(&self) -> String {
        match self.compose() {
            Source::Glsl(glsl, _) => glsl.to_string(),
            _ => panic!("ShaderComposer source is not GLSL"),
        }
    }

    fn shader(&self) -> Shader {
        match self.compose() {
            Source::Wgsl(wgsl) => Shader::from_wgsl(wgsl),
            Source::Glsl(glsl, stage) => Shader::from_glsl(glsl, stage),
            Source::SpirV(spirv) => Shader::from_spirv(spirv),
        }
    }
}

pub trait ShaderModule {
    fn name(&self) -> Cow<'static, str>;
    fn module(&self) -> Cow<'static, str>;

    fn emplace(&self, source: &mut Source) {
        source.emplace(&format!(r"module!\({:}\)", self.name()), &self.module());
    }
}

pub trait ShaderBlock {
    fn name(&self) -> &str;
    fn block(&self) -> &str;

    fn emplace(&self, source: &mut Source) {
        source.emplace(&format!(r"block!\({:}\)", self.name()), self.block());
    }
}

pub trait ShaderFunction {
    fn name(&self) -> &str;
    fn inputs(&self) -> Vec<(&str, &str)>;
    fn output(&self) -> Option<&str>;
    fn body(&self) -> &str;

    fn emplace(&self, source: &mut Source) {
        let mut ins_regex = String::default();
        let mut ins = String::default();
        for (name, ty) in self.inputs().iter() {
            ins_regex += &format!(r"[\s\r\n]*{name:}:[\s\r\n]*{ty:},?[\s\r\n]*");
            ins += &format!("    {name:}: {ty:},\n");
        }

        let regex = format!(
            r"#\[function\][\s\r\n]*fn[\s\r\n]*{:}[\s\r\n]*\({ins_regex:}\)[\s\r\n]*(->)?[\s\r\n]*[\w<>]*[\s\r\n]*\{{[\s\r\n]*(.*?)[\s\r\n]*\}}",
            self.name(),
        );

        let to = format!(
            "fn {:}(\n{ins:}) {:} {{\n{:}\n}}",
            self.name(),
            self.output()
                .map(|output| format!("-> {output:}"))
                .unwrap_or_default(),
            self.body()
        );

        source.emplace(&regex, &to);
    }
}

