use crossterm::{
    cursor::{Hide, MoveTo, MoveToColumn, MoveToNextLine, Show},
    event::{self, KeyCode, KeyModifiers},
    queue,
    style::{Print, PrintStyledContent, ResetColor, Stylize},
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, DisableLineWrap, EnableLineWrap,
    },
};
use std::io::{stdout, Write};
use termbg::Theme;

use crate::dependency::{Dependencies, Dependency, DependencyKind};

pub struct State {
    stdout: std::io::Stdout,
    selected: Vec<bool>,
    cursor_location: usize,
    outdated_deps: Dependencies,
    total_deps: usize,
    longest_attributes: Longest,
    theme: termbg::Theme,
}

pub enum Event {
    HandleKeyboard,
    UpdateDependencies,
    Exit,
}

struct Longest {
    name: usize,
    current_version: usize,
    latest_version: usize,
}

impl Longest {
    fn get_longest_attributes(dependencies: &Dependencies) -> Longest {
        let mut name = 0;
        let mut current_version = 0;
        let mut latest_version = 0;

        for dep in dependencies.iter() {
            name = name.max(dep.name.len());
            current_version = current_version.max(dep.current_version.len());
            latest_version = latest_version.max(dep.latest_version.len());
        }

        Longest {
            name,
            current_version,
            latest_version,
        }
    }
}

impl State {
    pub fn new(
        outdated_deps: Dependencies,
        total_deps: usize,
        default_selected: bool,
        theme: Theme,
    ) -> Self {
        Self {
            stdout: stdout(),
            selected: vec![default_selected; outdated_deps.len()],
            cursor_location: 0,
            longest_attributes: Longest::get_longest_attributes(&outdated_deps),
            outdated_deps,
            total_deps,
            theme,
        }
    }

    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        queue!(self.stdout, Hide, Clear(ClearType::All))?;

        self.render_header()?;
        self.render_dependencies(&[])?;
        self.render_footer_actions()?;

        self.stdout.flush()?;
        Ok(())
    }

    pub fn handle_keyboard_event(&mut self) -> Result<Event, Box<dyn std::error::Error>> {
        if let event::Event::Key(key) = event::read()? {
            match (key.code, key.modifiers) {
                (KeyCode::Up | KeyCode::Char('k'), _) => {
                    let prev_i = self.cursor_location;
                    self.cursor_location = if self.cursor_location == 0 {
                        self.outdated_deps.len() - 1
                    } else {
                        self.cursor_location - 1
                    };

                    self.render_dependencies(&[prev_i, self.cursor_location])?;
                }
                (KeyCode::Down | KeyCode::Char('j'), _) => {
                    let prev_i = self.cursor_location;
                    self.cursor_location = (self.cursor_location + 1) % self.outdated_deps.len();

                    self.render_dependencies(&[prev_i, self.cursor_location])?;
                }
                (KeyCode::Left | KeyCode::Char('h') | KeyCode::BackTab, _) => {
                    let prev_i = self.cursor_location;

                    self.cursor_location = self.prev_section_location();
                    self.render_dependencies(&[prev_i, self.cursor_location])?;
                }
                (KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab, _) => {
                    let prev_i = self.cursor_location;

                    self.cursor_location = self.next_section_location();
                    self.render_dependencies(&[prev_i, self.cursor_location])?;
                }
                (KeyCode::Char(' '), _) => {
                    self.selected[self.cursor_location] = !self.selected[self.cursor_location];
                    self.render_dependencies(&[self.cursor_location])?;
                }
                (KeyCode::Enter, _) => {
                    self.reset_terminal()?;
                    return Ok(Event::UpdateDependencies);
                }
                (KeyCode::Char('a'), _) => {
                    if self.selected.iter().all(|s| *s) {
                        self.selected = vec![false; self.outdated_deps.len()];
                    } else {
                        self.selected = vec![true; self.outdated_deps.len()];
                    }
                    self.render_dependencies(&[])?;
                }
                (KeyCode::Char('i'), _) => {
                    self.selected = self.selected.iter().map(|s| !s).collect();
                    self.render_dependencies(&[])?;
                }
                (KeyCode::Esc | KeyCode::Char('q'), _)
                | (KeyCode::Char('c') | KeyCode::Char('z'), KeyModifiers::CONTROL) => {
                    self.reset_terminal()?;
                    return Ok(Event::Exit);
                }
                _ => {}
            }
        }

        self.stdout.flush()?;
        Ok(Event::HandleKeyboard)
    }

    fn next_section_location(&self) -> usize {
        let curr_kind = self.outdated_deps.0[self.cursor_location].kind;

        let i = self
            .outdated_deps
            .0
            .iter()
            .enumerate()
            .skip_while(|(_, d)| d.kind != curr_kind)
            .skip_while(|(_, d)| d.kind == curr_kind)
            .map(|(i, _)| i)
            .next();

        if let Some(i) = i {
            i
        } else if self.outdated_deps.0[0].kind == self.outdated_deps.0[self.cursor_location].kind {
            self.cursor_location
        } else {
            0
        }
    }

    fn prev_section_location(&self) -> usize {
        let curr_kind = self.outdated_deps.0[self.cursor_location].kind;

        let mut iter = self.outdated_deps.0[..self.cursor_location]
            .iter()
            .enumerate()
            .rev()
            .skip_while(|(_, d)| d.kind == curr_kind);

        if let Some((i_, d_)) = iter.next() {
            iter.take_while(|(_, d)| d.kind == d_.kind)
                .last()
                .map(|(i, _)| i)
                .unwrap_or(i_)
        } else if self.outdated_deps.0.last().unwrap().kind
            == self.outdated_deps.0[self.cursor_location].kind
        {
            self.cursor_location
        } else {
            let last_kind = self.outdated_deps.0.last().unwrap().kind;

            self.outdated_deps
                .0
                .iter()
                .enumerate()
                .rev()
                .take_while(|(_, d)| d.kind == last_kind)
                .last()
                .unwrap()
                .0
        }
    }

    fn reset_terminal(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        queue!(self.stdout, Show, ResetColor)?;
        disable_raw_mode()?;
        Ok(())
    }

    pub fn selected_dependencies(self) -> Dependencies {
        Dependencies::new(
            self.outdated_deps
                .into_iter()
                .zip(self.selected.iter())
                .filter(|(_, s)| **s)
                .map(|(d, _)| d)
                .collect(),
        )
    }

    fn render_header(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        queue!(
            self.stdout,
            MoveTo(0, 0),
            Print(format!(
                "{} out of the {} direct dependencies are outdated.",
                self.outdated_deps.len().to_string().bold(),
                self.total_deps.to_string().bold()
            )),
            MoveToNextLine(1)
        )?;
        Ok(())
    }

    // parameter takes a specific indices to re-render
    // if indices is empty, then re-render the entire list
    fn render_dependencies(&mut self, indices: &[usize]) -> Result<(), Box<dyn std::error::Error>> {
        let mut offset = 0;

        queue!(self.stdout, DisableLineWrap)?;
        queue!(self.stdout, MoveTo(0, 0))?;

        for kind in DependencyKind::ordered() {
            offset += self.render_dependencies_subsection(kind, offset, indices)?;
        }

        queue!(self.stdout, EnableLineWrap)?;

        Ok(())
    }

    // Renders dependencies of a section
    // Returns length of dependencies in the section
    fn render_dependencies_subsection(
        &mut self,
        kind: DependencyKind,
        offset: usize,
        indices: &[usize],
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let deps = self
            .outdated_deps
            .iter()
            .enumerate()
            .skip(offset)
            .take_while(|(_, dep)| dep.kind == kind)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();

        if deps.is_empty() {
            return Ok(0);
        }

        let title = get_dependencies_subsection_title(kind);
        let num_selected = self
            .selected
            .iter()
            .zip(self.outdated_deps.iter())
            .filter(|(selected, dep)| **selected && dep.kind == kind)
            .count();

        queue!(self.stdout, MoveToNextLine(2))?;
        let row = crossterm::cursor::position()
            .expect("should return cursor position")
            .1;

        if !indices.is_empty() {
            queue!(
                self.stdout,
                MoveToColumn(title.len() as u16 + 2),
                Clear(ClearType::UntilNewLine),
                PrintStyledContent(format!("{num_selected} selected):").cyan()),
                MoveToNextLine(1)
            )?;

            for &i in indices {
                if offset <= i && i < offset + deps.len() {
                    // header (3) + section header (3 * num_kind) + i
                    queue!(
                        self.stdout,
                        MoveTo(0, 1 + 2 * (1 + kind as u16) + i as u16),
                        Clear(ClearType::CurrentLine)
                    )?;
                    self.render_dependency(i)?;
                }
            }
        } else {
            queue!(
                self.stdout,
                MoveToColumn(0),
                Clear(ClearType::CurrentLine),
                PrintStyledContent(format!("{title} ({num_selected} selected):").cyan()),
                MoveToNextLine(1)
            )?;

            for i in &deps {
                queue!(self.stdout, Clear(ClearType::CurrentLine))?;
                self.render_dependency(*i)?;
            }
        }

        queue!(self.stdout, MoveTo(0, row + deps.len() as u16))?;

        Ok(deps.len())
    }

    fn render_footer_actions(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        queue!(
            self.stdout,
            MoveToNextLine(2),
            Print(format!(
                "Use {} to navigate, {} to select all, {} to invert, {} to select/deselect, {} to update, {}/{} to exit",
                "arrow keys/hjkl".cyan(),
                "<a>".cyan(),
                "<i>".cyan(),
                "<space>".cyan(),
                "<enter>".cyan(),
                "<esc>".cyan(), "<q>".cyan()
            ))
        )?;
        Ok(())
    }

    fn render_dependency(&mut self, i: usize) -> Result<(), Box<dyn std::error::Error>> {
        let Dependency {
            name,
            current_version,
            latest_version,
            repository,
            description,
            latest_version_date,
            current_version_date,
            ..
        } = &self.outdated_deps.0[i];

        let name_spacing = " ".repeat(self.longest_attributes.name - name.len());
        let current_version_spacing =
            " ".repeat(self.longest_attributes.current_version - current_version.len());
        let latest_version_spacing =
            " ".repeat(self.longest_attributes.latest_version - latest_version.len());

        let bullet = if self.selected[i] { "●" } else { "○" };

        let latest_version_date = get_date_from_datetime_string(latest_version_date.as_deref())
            .unwrap_or("none      ")
            .italic()
            .dim();
        let current_version_date = get_date_from_datetime_string(current_version_date.as_deref())
            .unwrap_or("none      ")
            .italic()
            .dim();

        let name = name.clone().bold();
        let mut repository = repository.as_deref().unwrap_or("none").underline_black();
        if self.theme == Theme::Dark {
            repository = repository.underline_white();
        }

        let description = description.as_deref().unwrap_or("").dim();

        let mut current_version = current_version.clone().bold().black();
        if self.theme == Theme::Dark {
            current_version = current_version.white();
        }

        let mut latest_version = latest_version.clone().bold().black();
        if self.theme == Theme::Dark {
            latest_version = latest_version.white();
        }

        let row = format!(
            "{bullet} {name}{name_spacing}  {current_version_date} {current_version}{current_version_spacing} -> {latest_version_date} {latest_version}{latest_version_spacing}  {repository} - {description}"
        );

        let colored_row = if i == self.cursor_location {
            row.green()
        } else if self.theme == Theme::Light {
            row.black()
        } else {
            row.white()
        };

        queue!(
            self.stdout,
            PrintStyledContent(colored_row),
            MoveToNextLine(1),
        )?;
        Ok(())
    }
}

fn get_date_from_datetime_string(datetime_string: Option<&str>) -> Option<&str> {
    datetime_string
        .and_then(|s| s.split_once('T'))
        .map(|(date, _)| date)
}

fn get_dependencies_subsection_title(kind: DependencyKind) -> &'static str {
    match kind {
        DependencyKind::Normal => "Dependencies",
        DependencyKind::Dev => "Dev dependencies",
        DependencyKind::Build => "Build dependencies",
        DependencyKind::Workspace => "Workspace dependencies",
    }
}
