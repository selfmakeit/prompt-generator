use std::{
    fmt::{self, Write},
    fs,
    path::PathBuf,
};

use clipboard::{ClipboardContext, ClipboardProvider};
use eframe::egui::*;
use serde::{Deserialize, Serialize};

fn main() {
    let prompt: Prompt = fs::read(Prompt::path())
        .ok()
        .and_then(|bytes| serde_yaml::from_slice(&bytes).ok())
        .unwrap_or_else(|| Prompt {
            text: String::new(),
            style: Choices::new(["ultra realistic", "lo-fi anime"]),
            themes: ["cyberpunk", "steampunk"].map(|s| (s.into(), false)).into(),
            color: Choices::new(["vibrant", "muted", "grayscale", "high contrast"]),
            body: Choices::new(["feminine", "masculine"]),
            hair: Choices::new(["blonde", "brown", "black", "red", "light brown"]),
            pose: Choices::new(["dynamic", "relaxed", "confident"]),
            algorithm: Algorithm::V3,
            aspect: Aspect::Square,
            stylize: DEFAULT_STYLIZE,
            use_seed: false,
            seed: 0,
            video: false,
            copy_on_change: true,
            copied_command: String::new(),
        });
    let options = eframe::NativeOptions {
        min_window_size: Some([600.0, 400.0].into()),
        initial_window_size: Some([600.0, 700.0].into()),
        ..Default::default()
    };
    eframe::run_native(
        "Midjourney Prompt Generator",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_pixels_per_point(2.0);
            Box::new(prompt)
        }),
    );
}

#[derive(Serialize, Deserialize)]
struct Prompt {
    #[serde(skip)]
    text: String,
    style: Choices,
    themes: Vec<(String, bool)>,
    color: Choices,
    body: Choices,
    hair: Choices,
    pose: Choices,
    algorithm: Algorithm,
    aspect: Aspect,
    stylize: u32,
    video: bool,
    copy_on_change: bool,
    use_seed: bool,
    seed: u32,
    #[serde(skip)]
    copied_command: String,
}

const DEFAULT_STYLIZE: u32 = 2500;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Algorithm {
    V3,
    Test,
    TestPhoto,
}

impl Algorithm {
    fn str(&self) -> &'static str {
        match self {
            Algorithm::V3 => "v3",
            Algorithm::Test => "test",
            Algorithm::TestPhoto => "testp",
        }
    }
    fn allowed_aspects(&self) -> &'static [Aspect] {
        match self {
            Algorithm::V3 => &[
                Aspect::Square,
                Aspect::Portrait,
                Aspect::Landscape,
                Aspect::Tall,
                Aspect::Wide,
                Aspect::UltraWide,
            ],
            Algorithm::Test | Algorithm::TestPhoto => {
                &[Aspect::Square, Aspect::Portrait, Aspect::Landscape]
            }
        }
    }
}

impl fmt::Display for Algorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.str().fmt(f)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Aspect {
    Square,
    Portrait,
    Landscape,
    Tall,
    Wide,
    UltraWide,
}

impl Aspect {
    fn str(&self) -> &'static str {
        match self {
            Aspect::Square => "square",
            Aspect::Tall => "tall",
            Aspect::Portrait => "portrait",
            Aspect::Landscape => "landscape",
            Aspect::Wide => "wide",
            Aspect::UltraWide => "ultrawide",
        }
    }
    fn aspect_string(&self) -> String {
        let mut s = self.to_string();
        if let Some([w, h]) = self.wh() {
            write!(&mut s, " {w}:{h}").unwrap();
        }
        s
    }
    fn wh(&self) -> Option<[u8; 2]> {
        Some(match self {
            Aspect::Square => return None,
            Aspect::Portrait => [2, 3],
            Aspect::Landscape => [3, 2],
            Aspect::Tall => [1, 2],
            Aspect::Wide => [16, 9],
            Aspect::UltraWide => [21, 9],
        })
    }
}

impl fmt::Display for Aspect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.str().fmt(f)
    }
}

#[derive(Serialize, Deserialize)]
struct Choices {
    curr: Option<String>,
    choices: Vec<String>,
}

impl Prompt {
    fn dir() -> PathBuf {
        dirs::data_local_dir().unwrap().join("midjourney_prompt")
    }
    fn path() -> PathBuf {
        Self::dir().join("promt.yaml")
    }
    #[allow(unused_must_use)]
    fn command(&self) -> String {
        let mut s = format!("/imagine prompt: {}", self.text.trim());
        if let Some(style) = &self.style.curr {
            write!(&mut s, ", {}", style.trim());
        }
        if let Some(body) = &self.body.curr {
            write!(&mut s, ", {} body", body.trim());
        }
        if let Some(hair) = &self.hair.curr {
            write!(&mut s, ", {} hair", hair.trim());
        }
        if let Some(pose) = &self.pose.curr {
            write!(&mut s, ", {} pose", pose.trim());
        }
        for (theme, enabled) in &self.themes {
            if *enabled && !theme.trim().is_empty() {
                write!(&mut s, ", {}", theme.trim());
            }
        }
        if let Some(color) = &self.color.curr {
            write!(&mut s, ", {} colors", color.trim());
        }
        if self.stylize != DEFAULT_STYLIZE {
            write!(&mut s, " --stylize {}", self.stylize);
        }
        if let Some([w, h]) = self.aspect.wh() {
            write!(&mut s, " --ar {}:{}", w, h);
        }
        if self.video {
            s.push_str(" --video");
        }
        if self.use_seed {
            write!(&mut s, " --sameseed {}", self.seed);
        }
        if self.algorithm != Algorithm::V3 {
            write!(&mut s, " --{}", self.algorithm);
        }
        s
    }
}

impl eframe::App for Prompt {
    fn on_close_event(&mut self) -> bool {
        let _ = fs::create_dir_all(Self::dir());
        let _ = fs::write(Self::path(), serde_yaml::to_string(self).unwrap());
        true
    }
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let old_command = self.command();
        CentralPanel::default().show(ctx, |ui| {
            // Settings
            CollapsingHeader::new("settings").show(ui, |ui| {
                Grid::new("settings").show(ui, |ui| {
                    let cot_hover_text = "copy command to clipboard when changed";
                    ui.label("copy on change").on_hover_text(cot_hover_text);
                    ui.checkbox(&mut self.copy_on_change, "")
                        .on_hover_text(cot_hover_text);
                    ui.end_row();
                });
            });
            ui.separator();
            ScrollArea::both()
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    // Prompt
                    ui.label("prompt");
                    TextEdit::multiline(&mut self.text)
                        .show(ui)
                        .response
                        .changed();
                    // Basic
                    self.basic_ui(ui);
                    // Character
                    self.character_ui(ui);
                    // Command
                    ui.label("");
                    ui.horizontal_wrapped(|ui| {
                        ui.label(&self.copied_command);
                    });
                    let copy_to_clipboard = self.copy_on_change && self.command() != old_command
                        || !self.copy_on_change
                            && ui
                                .add_enabled(!self.text.trim().is_empty(), Button::new("copy"))
                                .clicked();
                    if copy_to_clipboard && !self.text.trim().is_empty() {
                        self.copied_command = match ClipboardContext::new()
                            .unwrap()
                            .set_contents(self.command())
                        {
                            Ok(()) => {
                                format!("copied command:\n{}", self.command())
                            }
                            Err(e) => format!("error copying command: {e}"),
                        };
                    }
                });
        });
    }
}

impl Prompt {
    fn basic_ui(&mut self, ui: &mut Ui) {
        Grid::new("basic").show(ui, |ui| {
            // Algorithm
            ui.label("algorithm");
            ui.horizontal(|ui| {
                for algo in [Algorithm::V3, Algorithm::Test, Algorithm::TestPhoto] {
                    if ui
                        .selectable_value(&mut self.algorithm, algo, algo.str())
                        .clicked()
                        && !self.algorithm.allowed_aspects().contains(&self.aspect)
                    {
                        self.aspect = match self.aspect {
                            Aspect::Tall => Aspect::Portrait,
                            Aspect::Wide | Aspect::UltraWide => Aspect::Landscape,
                            _ => self.aspect,
                        };
                    }
                }
            });
            ui.end_row();

            // Aspect
            ui.label("aspect");
            ComboBox::from_id_source("aspect")
                .selected_text(self.aspect.aspect_string())
                .width(100.0)
                .show_ui(ui, |ui| {
                    for aspect in self.algorithm.allowed_aspects() {
                        ui.selectable_value(&mut self.aspect, *aspect, aspect.aspect_string());
                    }
                });
            ui.end_row();

            // Stylize
            ui.label("stylize");
            Slider::new(&mut self.stylize, 625..=60000)
                .logarithmic(true)
                .show_value(false)
                .ui(ui);
            ui.horizontal(|ui| {
                DragValue::new(&mut self.stylize)
                    .clamp_range(625..=60000)
                    .ui(ui);
                if self.stylize != DEFAULT_STYLIZE && ui.button("reset").clicked() {
                    self.stylize = DEFAULT_STYLIZE;
                }
            });
            ui.end_row();

            // Seed
            ui.label("seed");
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.use_seed, "");
                if self.use_seed {
                    DragValue::new(&mut self.seed).ui(ui);
                }
            });
            ui.end_row();

            // Video
            ui.label("video");
            ui.checkbox(&mut self.video, "");
            ui.end_row();

            // Style
            self.style.row_ui(ui, "style");

            // Color
            self.color.row_ui(ui, "color");

            // Themes
            ui.label("themes");
            let mut enabled_themes = String::new();
            for (theme, enabled) in &self.themes {
                if *enabled && !theme.trim().is_empty() {
                    if !enabled_themes.is_empty() {
                        enabled_themes.push_str(", ");
                    }
                    enabled_themes.push_str(theme.trim());
                }
            }
            ui.horizontal_wrapped(|ui| ui.label(enabled_themes));
            CollapsingHeader::new("edit")
                .id_source("edit")
                .show(ui, |ui| {
                    for i in 0..self.themes.len() {
                        let removed = ui
                            .horizontal(|ui| {
                                let (theme, enabled) = &mut self.themes[i];
                                TextEdit::singleline(theme).desired_width(100.0).ui(ui);
                                ui.checkbox(enabled, "");
                                ui.button("-").clicked()
                            })
                            .inner;
                        if removed {
                            self.themes.remove(i);
                            break;
                        }
                    }
                    if ui.button("+").clicked() {
                        self.themes.push((String::new(), true));
                    }
                });
            ui.end_row();
        });
    }
    fn character_ui(&mut self, ui: &mut Ui) {
        CollapsingHeader::new("character")
            .id_source("character")
            .show(ui, |ui| {
                Grid::new("character").show(ui, |ui| {
                    self.body.row_ui(ui, "body");
                    self.hair.row_ui(ui, "hair color");
                    self.pose.row_ui(ui, "pose");
                });
            });
    }
}

impl Choices {
    fn new<'a>(choices: impl IntoIterator<Item = &'a str>) -> Self {
        Choices {
            curr: None,
            choices: choices.into_iter().map(Into::into).collect(),
        }
    }
    fn row_ui(&mut self, ui: &mut Ui, name: &str) {
        ui.label(name);
        ComboBox::from_id_source(name)
            .selected_text(self.curr.as_deref().unwrap_or("none"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.curr, None, "none");
                for style in self.choices.iter().filter(|s| !s.is_empty()) {
                    ui.selectable_value(&mut self.curr, Some(style.clone()), style);
                }
            });
        CollapsingHeader::new("edit")
            .id_source(name)
            .show(ui, |ui| {
                for i in 0..self.choices.len() {
                    let removed = ui
                        .horizontal(|ui| {
                            let style = &mut self.choices[i];
                            TextEdit::singleline(style).desired_width(100.0).show(ui);
                            self.choices.len() > 1 && ui.button("-").clicked()
                        })
                        .inner;
                    if removed {
                        self.choices.remove(i);
                        break;
                    }
                }
                if ui.button("+").clicked() {
                    self.choices.push(String::new());
                }
            });
        ui.end_row();
    }
}
