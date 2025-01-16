use crossterm::{
    cursor::{Hide, MoveTo, MoveToNextLine, Show},
    event::{self, KeyCode, KeyModifiers},
    execute,
    style::{Print, PrintStyledContent, ResetColor, Stylize},
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, DisableLineWrap, EnableLineWrap,
    },
};
use std::io::{stdout, Write};

use crate::dependency::{Dependencies, Dependency, DependencyKind};

pub struct State {
    stdout: std::io::Stdout,
    selected: Vec<bool>,
    cursor_location: usize,
    outdated_deps: Dependencies,
    total_deps: usize,
    longest_attributes: Longest,
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
    package_name: usize,
}

impl Longest {
    fn get_longest_attributes(dependencies: &Dependencies) -> Longest {
        let mut name = 0;
        let mut current_version = 0;
        let mut latest_version = 0;
        let mut package_name = 0;

        for dep in dependencies.iter() {
            name = name.max(dep.name.len());
            current_version = current_version.max(dep.current_version.len());
            latest_version = latest_version.max(dep.latest_version.len());
            package_name = package_name.max(dep.package_name.as_ref().map_or(0, |s| s.len()));
        }

        Longest {
            name,
            current_version,
            latest_version,
            package_name,
        }
    }
}

impl State {
    pub fn new(outdated_deps: Dependencies, total_deps: usize, default_selected: bool) -> Self {
        Self {
            stdout: stdout(),
            selected: vec![default_selected; outdated_deps.len()],
            cursor_location: 0,
            longest_attributes: Longest::get_longest_attributes(&outdated_deps),
            outdated_deps,
            total_deps,
        }
    }

    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        execute!(self.stdout, Hide)?;
        Ok(())
    }

    pub fn handle_keyboard_event(&mut self) -> Result<Event, Box<dyn std::error::Error>> {
        if let event::Event::Key(key) = event::read()? {
            match (key.code, key.modifiers) {
                (KeyCode::Up | KeyCode::Left, _) => {
                    self.cursor_location = if self.cursor_location == 0 {
                        self.outdated_deps.len() - 1
                    } else {
                        self.cursor_location - 1
                    };
                }
                (KeyCode::Down | KeyCode::Right, _) => {
                    self.cursor_location = (self.cursor_location + 1) % self.outdated_deps.len();
                }
                (KeyCode::Char(' '), _) => {
                    self.selected[self.cursor_location] = !self.selected[self.cursor_location];
                }
                (KeyCode::Enter, _) => {
                    self.reset_terminal()?;
                    return Ok(Event::UpdateDependencies);
                }
                (KeyCode::Char('a'), _) => {
                    self.selected = vec![true; self.outdated_deps.len()];
                }
                (KeyCode::Char('i'), _) => {
                    self.selected = self.selected.iter().map(|s| !s).collect();
                }
                (KeyCode::Esc | KeyCode::Char('q'), _)
                | (KeyCode::Char('c') | KeyCode::Char('z'), KeyModifiers::CONTROL) => {
                    self.reset_terminal()?;
                    return Ok(Event::Exit);
                }
                _ => {}
            }
        }

        Ok(Event::HandleKeyboard)
    }

    fn reset_terminal(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        execute!(self.stdout, Show, ResetColor)?;
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

    pub fn render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.render_header()?;
        self.render_dependencies()?;
        self.render_footer_actions()?;

        self.stdout.flush()?;
        Ok(())
    }

    fn render_header(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        execute!(
            self.stdout,
            Clear(ClearType::All),
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

    fn render_dependencies(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut offset = 0;

        execute!(self.stdout, DisableLineWrap)?;

        for kind in DependencyKind::ordered() {
            offset += self.render_dependencies_subsection(kind, offset)?;
        }

        execute!(self.stdout, EnableLineWrap)?;

        Ok(())
    }

    fn render_dependencies_subsection(
        &mut self,
        kind: DependencyKind,
        offset: usize,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let deps = self
            .outdated_deps
            .iter()
            .filter(|dep| dep.kind == kind)
            .cloned()
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

        execute!(
            self.stdout,
            MoveToNextLine(1),
            PrintStyledContent(format!("{title} ({num_selected} selected):").cyan()),
            MoveToNextLine(1)
        )?;

        for (i, dependency) in deps.iter().enumerate() {
            self.render_dependency(i + offset, dependency)?;
        }

        Ok(deps.len())
    }

    fn render_footer_actions(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        execute!(
            self.stdout,
            MoveToNextLine(2),
            Print(format!(
                "Use {} to navigate, {} to select all, {} to invert, {} to select/deselect, {} to update, {}/{} to exit",
                "arrow keys".cyan(),
                "<a>".cyan(),
                "<i>".cyan(),
                "<space>".cyan(),
                "<enter>".cyan(),
                "<esc>".cyan(), "<q>".cyan()
            ))
        )?;
        Ok(())
    }

    fn render_dependency(
        &mut self,
        i: usize,
        Dependency {
            name,
            current_version,
            latest_version,
            repository,
            description,
            latest_version_date,
            current_version_date,
            package_name,
            ..
        }: &Dependency,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let name_spacing = " ".repeat(self.longest_attributes.name - name.len());
        let current_version_spacing =
            " ".repeat(self.longest_attributes.current_version - current_version.len());
        let latest_version_spacing =
            " ".repeat(self.longest_attributes.latest_version - latest_version.len());

        let bullet = if self.selected[i] { "●" } else { "○" };

        let latest_version_date = get_date_from_datetime_string(latest_version_date.as_deref())
            .unwrap_or("none")
            .italic()
            .dim();
        let current_version_date = get_date_from_datetime_string(current_version_date.as_deref())
            .unwrap_or("none")
            .italic()
            .dim();

        let name = name.clone().bold();
        let repository = repository.as_deref().unwrap_or("none").underline_black();
        let description = description.as_deref().unwrap_or("").dim();
        let package_name = if self.outdated_deps.has_workspace_members() {
            let package_name = package_name.as_deref().unwrap_or("");
            let package_name = if package_name.is_empty() {
                "-".to_string()
            } else {
                package_name.to_string()
            };

            let package_name_spacing =
                " ".repeat(self.longest_attributes.package_name - package_name.len());
            format!("{package_name}{package_name_spacing}  ")
                .blue()
                .italic()
        } else {
            "".to_string().blue().italic()
        };

        let row = format!(
            "{bullet} {name}{name_spacing}  {package_name}{current_version_date} {current_version}{current_version_spacing} -> {latest_version_date} {latest_version}{latest_version_spacing}  {repository} - {description}",
        );

        let colored_row = if i == self.cursor_location {
            row.green()
        } else {
            row.black()
        };

        execute!(
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
