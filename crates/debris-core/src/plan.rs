use crate::model::AppxPackage;

/// Something about a step the user should know before it runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Caveat {
    /// Removal can break the shell, Search, the Store, or logon.
    SystemCritical,
    /// Microsoft's Store package. It cannot be reinstalled normally.
    Store,
}

impl Caveat {
    pub fn label(self) -> &'static str {
        match self {
            Self::SystemCritical => "SYSTEM",
            Self::Store => "IRREPLACEABLE",
        }
    }

    pub fn detail(self) -> &'static str {
        match self {
            Self::SystemCritical => "Windows needs this. Removing it can break the shell.",
            Self::Store => "Microsoft Store cannot be reinstalled the normal way.",
        }
    }
}

/// One package in the plan. Carries the full name, because that is what the
/// uninstall actually keys on — the display name is not enough to act.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemovalStep {
    pub name: String,
    pub full_name: String,
    pub caveat: Option<Caveat>,
}

/// What the ledger would do, as data. Nothing here executes anything: this is the
/// value the interface renders in the confirm step, and the value the backend turns
/// into commands. Building it is free and side-effect-free, which is the point.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RemovalPlan {
    steps: Vec<RemovalStep>,
}

impl RemovalPlan {
    pub fn from_packages(packages: &[AppxPackage]) -> Self {
        let steps = packages
            .iter()
            .map(|package| RemovalStep {
                name: package.name.clone(),
                full_name: package.full_name.clone(),
                caveat: caveat_for(package),
            })
            .collect();
        Self { steps }
    }

    pub fn steps(&self) -> &[RemovalStep] {
        &self.steps
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn caveat_count(&self) -> usize {
        self.steps.iter().filter(|step| step.caveat.is_some()).count()
    }

    /// True when the plan contains something irreversible or dangerous, so the
    /// interface must ask a second, explicit time before running it.
    pub fn requires_acknowledgement(&self) -> bool {
        self.caveat_count() > 0
    }

    /// The one line above the confirm control.
    pub fn headline(&self) -> String {
        match self.steps.len() {
            0 => "Nothing staged".to_string(),
            1 => "Remove 1 package".to_string(),
            count => format!("Remove {count} packages"),
        }
    }

    /// The second line, only when there is something to say.
    pub fn caveat_note(&self) -> Option<String> {
        let count = self.caveat_count();
        if count == 0 {
            return None;
        }
        Some(match count {
            1 => "1 of these is system-critical or cannot be reinstalled.".to_string(),
            count => format!("{count} of these are system-critical or cannot be reinstalled."),
        })
    }
}

fn caveat_for(package: &AppxPackage) -> Option<Caveat> {
    // The Store caveat is checked first: it is the more specific fact, and telling
    // someone "system" when the real answer is "you cannot get this back" is worse.
    if package.name.eq_ignore_ascii_case("Microsoft.WindowsStore") {
        return Some(Caveat::Store);
    }
    package.is_system.then_some(Caveat::SystemCritical)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(name: &str, system: bool) -> AppxPackage {
        AppxPackage::new(name, format!("{name}_1.0.0.0_x64__abc")).classify(system)
    }

    #[test]
    fn an_empty_staging_set_produces_an_empty_plan() {
        let plan = RemovalPlan::from_packages(&[]);
        assert!(plan.is_empty());
        assert_eq!(plan.len(), 0);
        assert_eq!(plan.headline(), "Nothing staged");
        assert!(plan.caveat_note().is_none());
        assert!(!plan.requires_acknowledgement());
    }

    #[test]
    fn the_headline_counts_correctly() {
        assert_eq!(
            RemovalPlan::from_packages(&[package("A", false)]).headline(),
            "Remove 1 package"
        );
        assert_eq!(
            RemovalPlan::from_packages(&[package("A", false), package("B", false)]).headline(),
            "Remove 2 packages"
        );
    }

    #[test]
    fn ordinary_packages_carry_no_caveat() {
        let plan = RemovalPlan::from_packages(&[package("SpotifyAB.SpotifyMusic", false)]);
        assert_eq!(plan.caveat_count(), 0);
        assert!(!plan.requires_acknowledgement());
    }

    #[test]
    fn system_packages_are_flagged() {
        let plan = RemovalPlan::from_packages(&[package("Microsoft.Windows.ShellExperienceHost", true)]);
        assert_eq!(plan.steps()[0].caveat, Some(Caveat::SystemCritical));
        assert!(plan.requires_acknowledgement());
        assert_eq!(
            plan.caveat_note().as_deref(),
            Some("1 of these is system-critical or cannot be reinstalled.")
        );
    }

    #[test]
    fn the_store_is_reported_as_irreplaceable_rather_than_system() {
        let plan = RemovalPlan::from_packages(&[package("Microsoft.WindowsStore", true)]);
        assert_eq!(plan.steps()[0].caveat, Some(Caveat::Store));
    }

    #[test]
    fn the_caveat_note_pluralises() {
        let plan = RemovalPlan::from_packages(&[
            package("Microsoft.WindowsStore", true),
            package("Microsoft.LockApp", true),
        ]);
        assert_eq!(
            plan.caveat_note().as_deref(),
            Some("2 of these are system-critical or cannot be reinstalled.")
        );
    }

    #[test]
    fn a_step_keeps_the_full_name_needed_to_uninstall() {
        let plan = RemovalPlan::from_packages(&[package("A", false)]);
        assert_eq!(plan.steps()[0].full_name, "A_1.0.0.0_x64__abc");
    }
}
