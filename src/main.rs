mod dsp;
mod ontology;
mod parser;
mod ui;

use crate::dsp::{AudioPlayer, Soundsculptor};
use crate::ontology::{EffectNode, Soundcourse};
use crate::parser::SoundcourseParser;
use crate::ui::{StyleManager, Symbols};
use clap::Parser;
use color_eyre::Result;
use directories::ProjectDirs;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyModifiers},
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Clear, Gauge, List, ListItem, ListState, Padding, Paragraph,
        Sparkline, Wrap,
    },
    DefaultTerminal, Frame,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::codecs::CODEC_TYPE_NULL;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

// --- CLI Arguments ---
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Optional path to a file or directory to open on startup
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,
}

// --- Persistence ---
#[derive(Serialize, Deserialize, Default)]
struct AppState {
    last_opened_path: Option<PathBuf>,
}

// --- Pane Enum ---
#[derive(PartialEq, Clone, Copy)]
enum Pane {
    FileExplorer,
    EffectPalette,
    Sequence,
    Parameters,
    SavePrompt,
    DirectoryInput,
    Processing,
}

// --- Spinner frames ---
const SPINNER: [&str; 12] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "⠋", "⠙"];

struct AmiximaApp {
    should_quit: bool,
    focused_pane: Pane,

    // File Explorer
    current_dir: Option<PathBuf>,
    files: Vec<String>,
    file_list_state: ListState,

    // Effect Palette
    effects_palette: Vec<String>,
    palette_state: ListState,

    // Soundcourse Sequence
    soundcourse: Soundcourse,
    sequence_state: ListState,
    parameter_state: ListState,

    // Directory Input
    dir_input: String,

    // Save Prompt
    save_input: String,

    // Processing state
    processing_file: String,
    processing_progress: f32,
    spinner_frame: usize,

    // Help
    show_help: bool,
    help_scroll: u16,

    // UI & Visuals
    style_manager: StyleManager,
    peak_data: Vec<u64>,
    audio_player: AudioPlayer,
}

impl AmiximaApp {
    fn new(start_path: Option<PathBuf>) -> Self {
        let mut app = Self {
            should_quit: false,
            focused_pane: Pane::FileExplorer,
            current_dir: None,
            files: Vec::new(),
            file_list_state: ListState::default(),
            effects_palette: vec![
                "EQ".to_string(),
                "Reverb".to_string(),
                "Delay".to_string(),
                "Compressor".to_string(),
                "Gain".to_string(),
            ],
            palette_state: ListState::default(),
            soundcourse: Soundcourse::new("User"),
            sequence_state: ListState::default(),
            parameter_state: ListState::default(),
            dir_input: String::new(),
            save_input: String::new(),
            processing_file: String::new(),
            processing_progress: 0.0,
            spinner_frame: 0,
            show_help: false,
            help_scroll: 0,
            style_manager: StyleManager::new(),
            peak_data: Vec::new(),
            audio_player: AudioPlayer::new(),
        };

        if let Some(path) = start_path {
            app.set_path(path);
        } else if let Some(last_path) = Self::load_persistent_state() {
            if last_path.exists() {
                app.set_path(last_path);
            }
        }

        app.update_peak_data();
        app
    }

    fn set_path(&mut self, path: PathBuf) {
        if path.is_dir() {
            self.current_dir = Some(path);
            self.refresh_files();
        } else if let Some(parent) = path.parent() {
            self.current_dir = Some(parent.to_path_buf());
            self.refresh_files();
        }
    }

    fn toggle_preview(&mut self) {
        if self.audio_player.is_playing() {
            self.audio_player.stop();
        } else if let Some(dir) = &self.current_dir {
            if let Some(idx) = self.file_list_state.selected() {
                if let Some(filename) = self.files.get(idx) {
                    let path = dir.join(filename);
                    let sc = self.soundcourse.clone();
                    if let Ok((samples, rate, channels)) =
                        Soundsculptor::get_processed_samples(path.to_str().unwrap(), &sc)
                    {
                        let _ = self.audio_player.play_samples(samples, rate, channels);
                    }
                }
            }
        }
    }

    fn refresh_files(&mut self) {
        self.files.clear();
        if let Some(dir) = &self.current_dir {
            let mut entries_vec = Vec::new();
            
            // Add ".." if not at root
            if dir.parent().is_some() {
                entries_vec.push("..".to_string());
            }

            if let Ok(entries) = fs::read_dir(dir) {
                let mut files: Vec<String> = entries
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.file_name().to_string_lossy().into_owned())
                    .collect();
                
                files.sort_by(|a, b| {
                    let path_a = dir.join(a);
                    let path_b = dir.join(b);
                    if path_a.is_dir() && !path_b.is_dir() {
                        std::cmp::Ordering::Less
                    } else if !path_a.is_dir() && path_b.is_dir() {
                        std::cmp::Ordering::Greater
                    } else {
                        a.cmp(b)
                    }
                });
                entries_vec.extend(files);
            }
            
            self.files = entries_vec;
            
            if !self.files.is_empty() {
                self.file_list_state.select(Some(0));
            }
        }
    }

    fn get_config_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "amixima", "amixima")
            .map(|proj| proj.config_dir().join("state.json"))
    }

    fn load_persistent_state() -> Option<PathBuf> {
        let path = Self::get_config_path()?;
        let data = fs::read_to_string(path).ok()?;
        let state: AppState = serde_json::from_str(&data).ok()?;
        state.last_opened_path
    }

    fn save_persistent_state(&self) {
        if let Some(path) = Self::get_config_path() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let state = AppState {
                last_opened_path: self.current_dir.clone(),
            };
            if let Ok(data) = serde_json::to_string(&state) {
                let _ = fs::write(path, data);
            }
        }
    }

    fn update_peak_data(&mut self) {
        if let Some(dir) = &self.current_dir {
            if let Some(idx) = self.file_list_state.selected() {
                if let Some(filename) = self.files.get(idx) {
                    let path = dir.join(filename);
                    if path.is_file() {
                        if is_audio_file(&path) {
                            if let Ok(peaks) = self.calculate_peaks(&path) {
                                self.peak_data = peaks;
                                return;
                            }
                        }
                    }
                }
            }
        }
        self.peak_data = vec![0; 50];
    }

    fn calculate_peaks(&self, path: &PathBuf) -> Result<Vec<u64>> {
        let src = File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(src), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }
        let probed = symphonia::default::get_probe().format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?;
        let mut format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| color_eyre::eyre::eyre!("no audio track"))?;
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())?;
        let track_id = track.id;

        let mut samples = Vec::new();
        let mut count = 0;
        let max_samples = 50_000; // Reduced for responsiveness

        'decode: loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(_) => break,
            };
            if packet.track_id() != track_id {
                continue;
            }
            let decoded = decoder.decode(&packet)?;
            match decoded {
                AudioBufferRef::F32(buf) => {
                    for i in (0..buf.frames()).step_by(2) {
                        samples.push(buf.chan(0)[i].abs());
                        count += 1;
                        if count > max_samples {
                            break 'decode;
                        }
                    }
                }
                AudioBufferRef::S16(buf) => {
                    for i in (0..buf.frames()).step_by(2) {
                        samples.push((buf.chan(0)[i] as f32 / 32768.0).abs());
                        count += 1;
                        if count > max_samples {
                            break 'decode;
                        }
                    }
                }
                _ => {}
            }
        }

        if samples.is_empty() {
            return Ok(vec![0; 50]);
        }

        let chunk_size = (samples.len() / 60).max(1);
        let mut peaks = Vec::new();
        for chunk in samples.chunks(chunk_size) {
            let max = chunk.iter().fold(0.0f32, |a, &b| a.max(b));
            peaks.push((max * 100.0) as u64);
        }
        while peaks.len() < 60 {
            peaks.push(0);
        }
        Ok(peaks.into_iter().take(60).collect())
    }

    // --- Input Handling ---
    fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|f| self.draw(f))?;
            self.spinner_frame = self.spinner_frame.wrapping_add(1);

            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    // If help is open, handle help-specific keys
                    if self.show_help {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('?') => {
                                self.show_help = false;
                                self.help_scroll = 0;
                            }
                            KeyCode::Down => {
                                self.help_scroll += 1;
                            }
                            KeyCode::Up => {
                                self.help_scroll = self.help_scroll.saturating_sub(1);
                            }
                            _ => {}
                        }
                        continue;
                    }

                    // If in DirectoryInput, handle text input
                    if self.focused_pane == Pane::DirectoryInput {
                        match key.code {
                            KeyCode::Enter => {
                                if !self.dir_input.is_empty() {
                                    let path = PathBuf::from(&self.dir_input);
                                    if path.exists() {
                                        self.set_path(path);
                                    }
                                }
                                self.focused_pane = Pane::FileExplorer;
                                self.dir_input.clear();
                            }
                            KeyCode::Esc => {
                                self.focused_pane = Pane::FileExplorer;
                                self.dir_input.clear();
                            }
                            KeyCode::Char(c) => {
                                self.dir_input.push(c);
                            }
                            KeyCode::Backspace => {
                                self.dir_input.pop();
                            }
                            _ => {}
                        }
                        continue;
                    }

                    // If in SavePrompt, handle text input
                    if self.focused_pane == Pane::SavePrompt {
                        match key.code {
                            KeyCode::Enter => {
                                if !self.save_input.is_empty() {
                                    if let Some(dir) = &self.current_dir {
                                        let path = dir.join(&self.save_input);
                                        if self.save_input.ends_with(".ini") {
                                            let _ = SoundcourseParser::serialize_to_ini(
                                                &self.soundcourse,
                                                path.to_str().unwrap(),
                                            );
                                        } else {
                                            let filename = if self.save_input.ends_with(".json") {
                                                self.save_input.clone()
                                            } else {
                                                format!("{}.json", self.save_input)
                                            };
                                            let path = dir.join(filename);
                                            if let Ok(json) = self.soundcourse.to_json_ld() {
                                                let _ = fs::write(path, json);
                                            }
                                        }
                                        self.refresh_files();
                                    }
                                }
                                self.focused_pane = Pane::Sequence;
                                self.save_input.clear();
                            }
                            KeyCode::Esc => {
                                self.focused_pane = Pane::Sequence;
                                self.save_input.clear();
                            }
                            KeyCode::Char(c) => {
                                self.save_input.push(c);
                            }
                            KeyCode::Backspace => {
                                self.save_input.pop();
                            }
                            _ => {}
                        }
                        continue;
                    }

                    // Main keybindings
                    match key.code {
                        KeyCode::Char('q') => {
                            self.save_persistent_state();
                            self.should_quit = true;
                        }
                        KeyCode::Char('?') => {
                            self.show_help = true;
                        }
                                            KeyCode::Char('p') => {
                                                self.toggle_preview();
                                            }
                                            KeyCode::Char('o') => {
                                                self.focused_pane = Pane::DirectoryInput;
                                            }
                                            KeyCode::Char('s') => {                            self.focused_pane = Pane::SavePrompt;
                        }
                        KeyCode::Char('d') => {
                            if self.focused_pane == Pane::Sequence {
                                self.delete_node();
                            }
                        }
                        KeyCode::Tab => {
                            self.focused_pane = match self.focused_pane {
                                Pane::FileExplorer => Pane::EffectPalette,
                                Pane::EffectPalette => Pane::Sequence,
                                Pane::Sequence => Pane::Parameters,
                                Pane::Parameters => Pane::FileExplorer,
                                Pane::SavePrompt => Pane::FileExplorer,
                                Pane::DirectoryInput => Pane::FileExplorer,
                                Pane::Processing => Pane::FileExplorer,
                            };
                        }
                        KeyCode::BackTab => {
                            self.focused_pane = match self.focused_pane {
                                Pane::FileExplorer => Pane::Parameters,
                                Pane::EffectPalette => Pane::FileExplorer,
                                Pane::Sequence => Pane::EffectPalette,
                                Pane::Parameters => Pane::Sequence,
                                Pane::SavePrompt => Pane::FileExplorer,
                                Pane::DirectoryInput => Pane::FileExplorer,
                                Pane::Processing => Pane::FileExplorer,
                            };
                        }
                        KeyCode::Down => {
                            if key.modifiers.contains(KeyModifiers::SHIFT)
                                && self.focused_pane == Pane::Sequence
                            {
                                self.reorder_node(true);
                            } else {
                                self.handle_down();
                            }
                        }
                        KeyCode::Up => {
                            if key.modifiers.contains(KeyModifiers::SHIFT)
                                && self.focused_pane == Pane::Sequence
                            {
                                self.reorder_node(false);
                            } else {
                                self.handle_up();
                            }
                        }
                        KeyCode::Left => self.handle_left(),
                        KeyCode::Right => self.handle_right(),
                        KeyCode::Enter => {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                let _ = self.process_files(terminal);
                            } else {
                                self.handle_enter();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    fn delete_node(&mut self) {
        if let Some(idx) = self.sequence_state.selected() {
            if idx < self.soundcourse.sequence.len() {
                self.soundcourse.sequence.remove(idx);
                if self.soundcourse.sequence.is_empty() {
                    self.sequence_state.select(None);
                    self.parameter_state.select(None);
                } else if idx >= self.soundcourse.sequence.len() {
                    self.sequence_state
                        .select(Some(self.soundcourse.sequence.len() - 1));
                }
            }
        }
    }

    fn reorder_node(&mut self, down: bool) {
        if let Some(idx) = self.sequence_state.selected() {
            let len = self.soundcourse.sequence.len();
            if down {
                if idx < len - 1 {
                    self.soundcourse.sequence.swap(idx, idx + 1);
                    self.sequence_state.select(Some(idx + 1));
                }
            } else {
                if idx > 0 {
                    self.soundcourse.sequence.swap(idx, idx - 1);
                    self.sequence_state.select(Some(idx - 1));
                }
            }
        }
    }

    fn process_files(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        if let Some(dir) = self.current_dir.clone() {
            let output_dir = dir.join("output");
            if !output_dir.exists() {
                fs::create_dir_all(&output_dir)?;
            }

            let audio_files: Vec<String> = self
                .files
                .iter()
                .filter(|f| is_audio_file(&dir.join(f)))
                .cloned()
                .collect();

            if audio_files.is_empty() {
                return Ok(());
            }

            self.focused_pane = Pane::Processing;
            let total = audio_files.len();

            for (i, file_name) in audio_files.iter().enumerate() {
                self.processing_file = file_name.clone();
                self.processing_progress = (i as f32) / (total as f32);
                terminal.draw(|f| self.draw(f))?;

                let input_path = dir.join(file_name);
                let output_name = format!(
                    "{}_processed.wav",
                    PathBuf::from(file_name)
                        .file_stem()
                        .unwrap()
                        .to_str()
                        .unwrap()
                );
                let output_path = output_dir.join(output_name);

                Soundsculptor::apply_soundcourse(
                    input_path.to_str().unwrap(),
                    output_path.to_str().unwrap(),
                    &self.soundcourse,
                )?;
            }

            self.processing_progress = 1.0;
            terminal.draw(|f| self.draw(f))?;
            self.focused_pane = Pane::FileExplorer;
        }
        Ok(())
    }

    fn handle_left(&mut self) {
        if self.focused_pane == Pane::Parameters {
            if let Some(seq_idx) = self.sequence_state.selected() {
                if let Some(param_idx) = self.parameter_state.selected() {
                    if let Some(node) = self.soundcourse.sequence.get_mut(seq_idx) {
                        match node {
                            EffectNode::Reverb { room_size, dry_wet } => {
                                if param_idx == 0 {
                                    *room_size = (*room_size - 0.05).clamp(0.0, 1.0);
                                } else {
                                    *dry_wet = (*dry_wet - 0.05).clamp(0.0, 1.0);
                                }
                            }
                            EffectNode::EQ { frequency, gain } => {
                                if param_idx == 0 {
                                    *frequency = (*frequency - 50.0).max(20.0);
                                } else {
                                    *gain = (*gain - 1.0).clamp(-24.0, 24.0);
                                }
                            }
                            EffectNode::Delay { delay_ms, feedback } => {
                                if param_idx == 0 {
                                    *delay_ms = (*delay_ms - 10.0).max(0.0);
                                } else {
                                    *feedback = (*feedback - 0.05).clamp(0.0, 1.0);
                                }
                            }
                            EffectNode::Compressor { threshold, ratio } => {
                                if param_idx == 0 {
                                    *threshold = (*threshold - 1.0).clamp(-60.0, 0.0);
                                } else {
                                    *ratio = (*ratio - 0.1).max(1.0);
                                }
                            }
                            EffectNode::Gain { gain_db } => {
                                *gain_db -= 0.5;
                            }
                        }
                    }
                }
            }
        }
    }

    fn handle_right(&mut self) {
        if self.focused_pane == Pane::Parameters {
            if let Some(seq_idx) = self.sequence_state.selected() {
                if let Some(param_idx) = self.parameter_state.selected() {
                    if let Some(node) = self.soundcourse.sequence.get_mut(seq_idx) {
                        match node {
                            EffectNode::Reverb { room_size, dry_wet } => {
                                if param_idx == 0 {
                                    *room_size = (*room_size + 0.05).clamp(0.0, 1.0);
                                } else {
                                    *dry_wet = (*dry_wet + 0.05).clamp(0.0, 1.0);
                                }
                            }
                            EffectNode::EQ { frequency, gain } => {
                                if param_idx == 0 {
                                    *frequency = (*frequency + 50.0).min(20000.0);
                                } else {
                                    *gain = (*gain + 1.0).clamp(-24.0, 24.0);
                                }
                            }
                            EffectNode::Delay { delay_ms, feedback } => {
                                if param_idx == 0 {
                                    *delay_ms = (*delay_ms + 10.0).min(2000.0);
                                } else {
                                    *feedback = (*feedback + 0.05).clamp(0.0, 1.0);
                                }
                            }
                            EffectNode::Compressor { threshold, ratio } => {
                                if param_idx == 0 {
                                    *threshold = (*threshold + 1.0).clamp(-60.0, 0.0);
                                } else {
                                    *ratio = (*ratio + 0.1).min(20.0);
                                }
                            }
                            EffectNode::Gain { gain_db } => {
                                *gain_db += 0.5;
                            }
                        }
                    }
                }
            }
        }
    }

    fn handle_down(&mut self) {
        match self.focused_pane {
            Pane::FileExplorer => {
                let i = match self.file_list_state.selected() {
                    Some(i) => {
                        if i >= self.files.len().saturating_sub(1) {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.file_list_state.select(Some(i));
                self.update_peak_data();
            }
            Pane::EffectPalette => {
                let i = match self.palette_state.selected() {
                    Some(i) => {
                        if i >= self.effects_palette.len().saturating_sub(1) {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.palette_state.select(Some(i));
            }
            Pane::Sequence => {
                let i = match self.sequence_state.selected() {
                    Some(i) => {
                        if i >= self.soundcourse.sequence.len().saturating_sub(1) {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.sequence_state.select(Some(i));
            }
            Pane::Parameters => {
                if let Some(idx) = self.sequence_state.selected() {
                    if let Some(node) = self.soundcourse.sequence.get(idx) {
                        let num_params = match node {
                            EffectNode::Reverb { .. } => 2,
                            EffectNode::EQ { .. } => 2,
                            EffectNode::Delay { .. } => 2,
                            EffectNode::Compressor { .. } => 2,
                            EffectNode::Gain { .. } => 1,
                        };
                        let i = match self.parameter_state.selected() {
                            Some(i) => {
                                if i >= num_params - 1 {
                                    0
                                } else {
                                    i + 1
                                }
                            }
                            None => 0,
                        };
                        self.parameter_state.select(Some(i));
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_up(&mut self) {
        match self.focused_pane {
            Pane::FileExplorer => {
                let i = match self.file_list_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.files.len().saturating_sub(1)
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.file_list_state.select(Some(i));
                self.update_peak_data();
            }
            Pane::EffectPalette => {
                let i = match self.palette_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.effects_palette.len().saturating_sub(1)
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.palette_state.select(Some(i));
            }
            Pane::Sequence => {
                let i = match self.sequence_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.soundcourse.sequence.len().saturating_sub(1)
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.sequence_state.select(Some(i));
            }
            Pane::Parameters => {
                if let Some(idx) = self.sequence_state.selected() {
                    if let Some(node) = self.soundcourse.sequence.get(idx) {
                        let num_params = match node {
                            EffectNode::Reverb { .. } => 2,
                            EffectNode::EQ { .. } => 2,
                            EffectNode::Delay { .. } => 2,
                            EffectNode::Compressor { .. } => 2,
                            EffectNode::Gain { .. } => 1,
                        };
                        let i = match self.parameter_state.selected() {
                            Some(i) => {
                                if i == 0 {
                                    num_params - 1
                                } else {
                                    i - 1
                                }
                            }
                            None => 0,
                        };
                        self.parameter_state.select(Some(i));
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_enter(&mut self) {
        match self.focused_pane {
            Pane::FileExplorer => {
                if let Some(current) = &self.current_dir {
                    if let Some(idx) = self.file_list_state.selected() {
                        if let Some(filename) = self.files.get(idx) {
                            if filename == ".." {
                                if let Some(parent) = current.parent() {
                                    let parent_path = parent.to_path_buf();
                                    self.current_dir = Some(parent_path);
                                    self.refresh_files();
                                }
                                return;
                            }
                            let new_path = current.join(filename);
                            if new_path.is_dir() {
                                self.current_dir = Some(new_path);
                                self.refresh_files();
                            } else if filename.to_lowercase().ends_with(".ini") {
                                if let Ok(sc) =
                                    SoundcourseParser::parse_ini(new_path.to_str().unwrap())
                                {
                                    self.soundcourse = sc;
                                    if !self.soundcourse.sequence.is_empty() {
                                        self.sequence_state.select(Some(0));
                                    } else {
                                        self.sequence_state.select(None);
                                    }
                                    self.parameter_state.select(None);
                                }
                            }
                        }
                    }
                }
            }
            Pane::EffectPalette => {
                if let Some(idx) = self.palette_state.selected() {
                    if let Some(effect_name) = self.effects_palette.get(idx) {
                        let node = match effect_name.as_str() {
                            "Reverb" => EffectNode::Reverb {
                                room_size: 0.5,
                                dry_wet: 0.3,
                            },
                            "EQ" => EffectNode::EQ {
                                frequency: 1000.0,
                                gain: 0.0,
                            },
                            "Delay" => EffectNode::Delay {
                                delay_ms: 100.0,
                                feedback: 0.5,
                            },
                            "Compressor" => EffectNode::Compressor {
                                threshold: -20.0,
                                ratio: 4.0,
                            },
                            "Gain" => EffectNode::Gain { gain_db: 0.0 },
                            _ => return,
                        };
                        self.soundcourse.sequence.push(node);
                        self.sequence_state
                            .select(Some(self.soundcourse.sequence.len() - 1));
                    }
                }
            }
            _ => {}
        }
    }

    // --- Enhanced Draw Method ---
    fn draw(&mut self, f: &mut Frame) {
        let area = f.area();
        let main_chunks = Layout::vertical([
            Constraint::Length(3), // Title Bar
            Constraint::Min(0),    // Workspace
            Constraint::Length(3), // Status Bar
        ])
        .split(area);

        self.draw_title_bar(f, main_chunks[0]);
        self.draw_workspace(f, main_chunks[1]);
        self.draw_status_bar(f, main_chunks[2]);

        // Modals
        if self.focused_pane == Pane::SavePrompt {
            self.draw_save_modal(f);
        }
        if self.focused_pane == Pane::DirectoryInput {
            self.draw_directory_modal(f);
        }
        if self.focused_pane == Pane::Processing {
            self.draw_processing_modal(f);
        }
        if self.show_help {
            self.draw_help_modal(f);
        }
    }

    fn draw_title_bar(&self, f: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .border_set(ratatui::symbols::border::THICK)
            .border_style(Style::default().fg(self.style_manager.palette.accent_cyan))
            .bg(self.style_manager.palette.surface);
        
        f.render_widget(block, area);
        
        let inner = area.inner(Margin { horizontal: 2, vertical: 1 });
        let title_chunks = Layout::horizontal([
            Constraint::Length(25),
            Constraint::Min(0),
            Constraint::Length(30),
        ]).split(inner);

        let title = Text::from(vec![Line::from(vec![
            Span::styled(" AMIXIMA ", Style::default().fg(self.style_manager.palette.accent_fuchsia).bold()),
            Span::styled("⬡", Style::default().fg(self.style_manager.palette.accent_cyan)),
            Span::styled(" Audio Sculptor ", Style::default().fg(self.style_manager.palette.text_bright)),
        ])]);
        f.render_widget(Paragraph::new(title), title_chunks[0]);

        // Live Peak Meter
        let sparkline = Sparkline::default()
            .data(&self.peak_data)
            .style(Style::default().fg(self.style_manager.palette.accent_cyan));
        f.render_widget(sparkline, title_chunks[1]);

        let time_str = chrono::Local::now().format("%H:%M:%S").to_string();
        let info = Line::from(vec![
            Span::styled(format!(" {} ", Symbols::NAV_INDICATOR), Style::default().fg(self.style_manager.palette.accent_gold)),
            Span::styled(time_str, Style::default().fg(self.style_manager.palette.text_dim)),
        ]).right_aligned();
        f.render_widget(Paragraph::new(info), title_chunks[2]);
    }

    fn draw_workspace(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::horizontal([
            Constraint::Percentage(25), // File Explorer
            Constraint::Percentage(25), // Effect Palette
            Constraint::Percentage(50), // Sequence & Parameters
        ])
        .split(area);

        self.draw_file_explorer(f, chunks[0]);
        self.draw_effect_palette(f, chunks[1]);
        
        let right_chunks = Layout::vertical([
            Constraint::Percentage(60), // Sequence
            Constraint::Percentage(40), // Parameters
        ]).split(chunks[2]);
        
        self.draw_sequence(f, right_chunks[0]);
        self.draw_parameters(f, right_chunks[1]);
    }

    fn draw_file_explorer(&mut self, f: &mut Frame, area: Rect) {
        let active = self.focused_pane == Pane::FileExplorer;
        let block = Block::bordered()
            .title(Line::from(vec![
                Span::styled(" 1 ", Style::default().fg(self.style_manager.palette.accent_gold).bold()),
                Span::styled(" FILES ", self.style_manager.title_style(active)),
            ]))
            .border_type(self.style_manager.border_type(active))
            .border_style(self.style_manager.block_style(active))
            .padding(Padding::horizontal(1));

        if let Some(dir) = &self.current_dir {
            let items: Vec<ListItem> = self.files.iter().map(|f| {
                let path = dir.join(f);
                let (icon, color) = if path.is_dir() {
                    (Symbols::DIR_ICON, self.style_manager.palette.accent_cyan)
                } else if is_audio_file(&path) {
                    (Symbols::AUDIO_ICON, self.style_manager.palette.accent_green)
                } else if f.ends_with(".ini") || f.ends_with(".json") {
                    (Symbols::CONFIG_ICON, self.style_manager.palette.accent_gold)
                } else {
                    (Symbols::FILE_ICON, self.style_manager.palette.text_dim)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(icon, Style::default().fg(color)),
                    Span::raw(f),
                ]))
            }).collect();

            let list = List::new(items)
                .block(block)
                .highlight_style(self.style_manager.list_highlight_style(active))
                .highlight_symbol(Symbols::FOCUS_MARKER);
            f.render_stateful_widget(list, area, &mut self.file_list_state);
        } else {
            f.render_widget(Paragraph::new("No directory loaded").block(block), area);
        }
    }

    fn draw_effect_palette(&mut self, f: &mut Frame, area: Rect) {
        let active = self.focused_pane == Pane::EffectPalette;
        let block = Block::bordered()
            .title(Line::from(vec![
                Span::styled(" 2 ", Style::default().fg(self.style_manager.palette.accent_gold).bold()),
                Span::styled(" AUFX PALETTE ", self.style_manager.title_style(active)),
            ]))
            .border_type(self.style_manager.border_type(active))
            .border_style(self.style_manager.block_style(active))
            .padding(Padding::horizontal(1));

        let items: Vec<ListItem> = self.effects_palette.iter().map(|e| {
            ListItem::new(Line::from(vec![
                Span::styled(Symbols::ITEM_MARKER, Style::default().fg(self.style_manager.palette.accent_fuchsia)),
                Span::raw(e),
            ]))
        }).collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(self.style_manager.list_highlight_style(active))
            .highlight_symbol(Symbols::FOCUS_MARKER);
        f.render_stateful_widget(list, area, &mut self.palette_state);
    }

    fn draw_sequence(&mut self, f: &mut Frame, area: Rect) {
        let active = self.focused_pane == Pane::Sequence;
        let block = Block::bordered()
            .title(Line::from(vec![
                Span::styled(" 3 ", Style::default().fg(self.style_manager.palette.accent_gold).bold()),
                Span::styled(" SOUNDCOURSE ", self.style_manager.title_style(active)),
            ]))
            .border_type(self.style_manager.border_type(active))
            .border_style(self.style_manager.block_style(active))
            .padding(Padding::horizontal(1));

        let items: Vec<ListItem> = self.soundcourse.sequence.iter().enumerate().map(|(i, node)| {
            let label = match node {
                EffectNode::Reverb { .. } => "Reverb",
                EffectNode::EQ { .. } => "EQ",
                EffectNode::Delay { .. } => "Delay",
                EffectNode::Compressor { .. } => "Compressor",
                EffectNode::Gain { .. } => "Gain",
            };
            
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:02} ", i + 1), Style::default().fg(self.style_manager.palette.text_dim)),
                Span::styled(label, Style::default().bold()),
            ]))
        }).collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(self.style_manager.list_highlight_style(active))
            .highlight_symbol(Symbols::FOCUS_MARKER);
        f.render_stateful_widget(list, area, &mut self.sequence_state);
    }

    fn draw_parameters(&mut self, f: &mut Frame, area: Rect) {
        let active = self.focused_pane == Pane::Parameters;
        let block = Block::bordered()
            .title(Line::from(vec![
                Span::styled(" 4 ", Style::default().fg(self.style_manager.palette.accent_gold).bold()),
                Span::styled(" PARAMETERS ", self.style_manager.title_style(active)),
            ]))
            .border_type(self.style_manager.border_type(active))
            .border_style(self.style_manager.block_style(active))
            .padding(Padding::uniform(1));

        let mut param_list = Vec::new();
        if let Some(idx) = self.sequence_state.selected() {
            if let Some(node) = self.soundcourse.sequence.get(idx) {
                match node {
                    EffectNode::Reverb { room_size, dry_wet } => {
                        param_list.push(("Room Size", *room_size));
                        param_list.push(("Dry/Wet", *dry_wet));
                    }
                    EffectNode::EQ { frequency, gain } => {
                        param_list.push(("Frequency", frequency / 20000.0));
                        param_list.push(("Gain", (gain + 24.0) / 48.0));
                    }
                    EffectNode::Delay { delay_ms, feedback } => {
                        param_list.push(("Delay", delay_ms / 2000.0));
                        param_list.push(("Feedback", *feedback));
                    }
                    EffectNode::Compressor { threshold, ratio } => {
                        param_list.push(("Threshold", (threshold + 60.0) / 60.0));
                        param_list.push(("Ratio", (ratio - 1.0) / 19.0));
                    }
                    EffectNode::Gain { gain_db } => {
                        param_list.push(("Gain", (gain_db + 24.0) / 48.0));
                    }
                }
            }
        }

        let inner_area = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::vertical(
            vec![Constraint::Length(2); param_list.len()]
        ).split(inner_area);

        for (i, (name, val)) in param_list.iter().enumerate() {
            let is_selected = self.parameter_state.selected() == Some(i) && active;
            let color = if is_selected { self.style_manager.palette.accent_fuchsia } else { self.style_manager.palette.accent_cyan };
            
            let gauge = Gauge::default()
                .gauge_style(Style::default().fg(color).bg(self.style_manager.palette.selection_bg))
                .ratio(*val as f64)
                .label(format!("{} : {:.2}", name, val));
            f.render_widget(gauge, chunks[i]);
        }
    }

    fn draw_status_bar(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .bg(self.style_manager.palette.surface)
            .borders(Borders::TOP)
            .border_style(Style::default().fg(self.style_manager.palette.border_inactive));
        
        f.render_widget(block, area);
        
        let inner = area.inner(Margin { horizontal: 1, vertical: 1 });
        let chunks = Layout::horizontal([
            Constraint::Length(20),
            Constraint::Min(0),
        ]).split(inner);

        let pane_name = format!(" FOCUS: {} ", match self.focused_pane {
            Pane::FileExplorer => "FILES",
            Pane::EffectPalette => "PALETTE",
            Pane::Sequence => "SEQUENCE",
            Pane::Parameters => "PARAMETERS",
            Pane::SavePrompt => "SAVE",
            Pane::DirectoryInput => "OPEN DIR",
            Pane::Processing => "SCULPTING",
        });

        f.render_widget(Paragraph::new(Span::styled(pane_name, Style::default().bg(self.style_manager.palette.accent_cyan).fg(Color::Black).bold())), chunks[0]);

        let hints = Line::from(vec![
            Span::styled(" [Tab] ", Style::default().fg(self.style_manager.palette.accent_gold)),
            Span::raw("Cycle "),
            Span::styled(" [o] ", Style::default().fg(self.style_manager.palette.accent_gold)),
            Span::raw("Open Dir "),
            Span::styled(" [p] ", Style::default().fg(self.style_manager.palette.accent_gold)),
            Span::raw("Preview "),
            Span::styled(" [s] ", Style::default().fg(self.style_manager.palette.accent_gold)),
            Span::raw("Save "),
            Span::styled(" [Ctrl+Enter] ", Style::default().fg(self.style_manager.palette.accent_gold)),
            Span::raw("Process All "),
            Span::styled(" [?] ", Style::default().fg(self.style_manager.palette.accent_gold)),
            Span::raw("Help "),
        ]).right_aligned();
        f.render_widget(Paragraph::new(hints), chunks[1]);
    }

    fn draw_save_modal(&self, f: &mut Frame) {
        let area = centered_rect(50, 20, f.area());
        f.render_widget(Clear, area);
        
        let block = Block::bordered()
            .title(" SAVE SOUNDCOURSE ")
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(self.style_manager.palette.accent_gold))
            .bg(self.style_manager.palette.background)
            .padding(Padding::uniform(1));
        
        let inner = block.inner(area);
        f.render_widget(block, area);
        
        let text = vec![
            Line::from("Enter filename (.ini or .json):"),
            Line::from(""),
            Line::from(vec![
                Span::styled(Symbols::FOCUS_MARKER, Style::default().fg(self.style_manager.palette.accent_fuchsia)),
                Span::styled(&self.save_input, Style::default().fg(self.style_manager.palette.text_bright).underlined()),
            ]),
        ];
        f.render_widget(Paragraph::new(text), inner);
    }

    fn draw_directory_modal(&self, f: &mut Frame) {
        let area = centered_rect(60, 20, f.area());
        f.render_widget(Clear, area);
        
        let block = Block::bordered()
            .title(" OPEN DIRECTORY ")
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(self.style_manager.palette.accent_cyan))
            .bg(self.style_manager.palette.background)
            .padding(Padding::uniform(1));
        
        let inner = block.inner(area);
        f.render_widget(block, area);
        
        let text = vec![
            Line::from("Enter absolute path to audio directory:"),
            Line::from(""),
            Line::from(vec![
                Span::styled(Symbols::FOCUS_MARKER, Style::default().fg(self.style_manager.palette.accent_fuchsia)),
                Span::styled(&self.dir_input, Style::default().fg(self.style_manager.palette.text_bright).underlined()),
            ]),
        ];
        f.render_widget(Paragraph::new(text), inner);
    }

    fn draw_processing_modal(&self, f: &mut Frame) {
        let area = centered_rect(60, 25, f.area());
        f.render_widget(Clear, area);
        
        let block = Block::bordered()
            .title(" SCULPTING AUDIO ")
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(self.style_manager.palette.accent_cyan))
            .bg(self.style_manager.palette.background)
            .padding(Padding::uniform(2));
        
        let inner = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ]).split(inner);

        let spinner_char = SPINNER[self.spinner_frame % SPINNER.len()];
        let title = Line::from(vec![
            Span::styled(spinner_char, Style::default().fg(self.style_manager.palette.accent_fuchsia).bold()),
            Span::raw(" Processing: "),
            Span::styled(&self.processing_file, Style::default().fg(self.style_manager.palette.text_bright).italic()),
        ]);
        f.render_widget(Paragraph::new(title), chunks[0]);

        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(self.style_manager.palette.accent_green).bg(self.style_manager.palette.selection_bg))
            .percent((self.processing_progress * 100.0) as u16)
            .label(format!("{:.1}%", self.processing_progress * 100.0));
        f.render_widget(gauge, chunks[1]);
    }

    fn draw_help_modal(&self, f: &mut Frame) {
        let area = centered_rect(70, 70, f.area());
        f.render_widget(Clear, area);
        
        let block = Block::bordered()
            .title(" AMIXIMA COMMANDS ")
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(self.style_manager.palette.accent_green))
            .bg(self.style_manager.palette.background)
            .padding(Padding::uniform(1));
        
        let help_content = vec![
            Line::from(vec![Span::styled("Navigation", Style::default().fg(self.style_manager.palette.accent_fuchsia).bold())]),
            Line::from(vec![Span::styled("  Tab        ", Style::default().fg(self.style_manager.palette.accent_gold)), Span::raw("Cycle focus between panes")]),
            Line::from(vec![Span::styled("  ↑/↓        ", Style::default().fg(self.style_manager.palette.accent_gold)), Span::raw("Navigate lists / parameters")]),
            Line::from(vec![Span::styled("  Enter      ", Style::default().fg(self.style_manager.palette.accent_gold)), Span::raw("Select / Action")]),
            Line::from(""),
            Line::from(vec![Span::styled("Audio Controls", Style::default().fg(self.style_manager.palette.accent_fuchsia).bold())]),
            Line::from(vec![Span::styled("  p          ", Style::default().fg(self.style_manager.palette.accent_gold)), Span::raw("Toggle Preview playback")]),
            Line::from(vec![Span::styled("  Ctrl+Enter ", Style::default().fg(self.style_manager.palette.accent_gold)), Span::raw("Batch process all files")]),
            Line::from(""),
            Line::from(vec![Span::styled("Sequence Management", Style::default().fg(self.style_manager.palette.accent_fuchsia).bold())]),
            Line::from(vec![Span::styled("  Shift+↑/↓  ", Style::default().fg(self.style_manager.palette.accent_gold)), Span::raw("Move effect in sequence")]),
            Line::from(vec![Span::styled("  d          ", Style::default().fg(self.style_manager.palette.accent_gold)), Span::raw("Delete selected effect")]),
            Line::from(vec![Span::styled("  s          ", Style::default().fg(self.style_manager.palette.accent_gold)), Span::raw("Save current Soundcourse")]),
            Line::from(""),
            Line::from(vec![Span::styled("Global", Style::default().fg(self.style_manager.palette.accent_fuchsia).bold())]),
            Line::from(vec![Span::styled("  ?          ", Style::default().fg(self.style_manager.palette.accent_gold)), Span::raw("Toggle this help")]),
            Line::from(vec![Span::styled("  q          ", Style::default().fg(self.style_manager.palette.accent_gold)), Span::raw("Quit Amixima")]),
        ];

        let paragraph = Paragraph::new(help_content)
            .block(block)
            .wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
    }
}

fn is_audio_file(path: &Path) -> bool {
    let audio_extensions = ["wav", "mp3", "flac", "ogg", "m4a", "aiff"];
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        audio_extensions.contains(&ext.to_lowercase().as_str())
    } else {
        false
    }
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    r: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();

    ratatui::run(|terminal| {
        let mut app = AmiximaApp::new(args.path);
        app.run(terminal)
    })?;
    Ok(())
}
