//! Translates app→channel assignments into process taps (`docs/architecture/audio-engine.md` §4).
//!
//! Pure logic, compiled and tested on every OS.

use std::collections::BTreeMap;

use ss_core::mixer::ChannelId;

/// A running app identified by its Unix process id.
pub type Pid = i32;

/// What a single tap should capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TapSource {
    /// Only these processes.
    Include(Vec<Pid>),
    /// Every process except these (catch-all for unassigned apps).
    AllExcept(Vec<Pid>),
}

/// One tap and the mixer channel its audio feeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TapSpec {
    /// Destination channel.
    pub channel: ChannelId,
    /// Captured processes.
    pub source: TapSource,
}

/// Builds the tap list for the given assignments.
///
/// * Every non-Media channel with at least one app gets an `Include` tap.
/// * Media gets one `AllExcept` tap that excludes every app assigned to another channel, so all
///   unassigned apps (and apps explicitly assigned to Media) land there.
/// * `own_pid` (the StealSeries process itself) is always excluded to avoid feedback.
///
/// The returned order is the order of the aggregate device's input streams.
#[must_use]
pub fn plan_taps(assignments: &BTreeMap<Pid, ChannelId>, own_pid: Pid) -> Vec<TapSpec> {
    let mut per_channel: BTreeMap<ChannelId, Vec<Pid>> = BTreeMap::new();
    for (&pid, &channel) in assignments {
        if pid != own_pid && channel != ChannelId::Media {
            per_channel.entry(channel).or_default().push(pid);
        }
    }

    let mut excluded: Vec<Pid> = per_channel.values().flatten().copied().collect();
    excluded.push(own_pid);
    excluded.sort_unstable();

    let mut taps: Vec<TapSpec> = per_channel
        .into_iter()
        .map(|(channel, pids)| TapSpec {
            channel,
            source: TapSource::Include(pids),
        })
        .collect();
    taps.push(TapSpec {
        channel: ChannelId::Media,
        source: TapSource::AllExcept(excluded),
    });
    taps
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWN: Pid = 1;

    #[test]
    fn no_assignments_routes_everything_to_media() {
        let taps = plan_taps(&BTreeMap::new(), OWN);
        assert_eq!(
            taps,
            vec![TapSpec {
                channel: ChannelId::Media,
                source: TapSource::AllExcept(vec![OWN])
            }]
        );
    }

    #[test]
    fn assigned_apps_are_excluded_from_media() {
        let assignments = BTreeMap::from([
            (100, ChannelId::Game),
            (200, ChannelId::Chat),
            (201, ChannelId::Chat),
            (300, ChannelId::Media),
        ]);
        let taps = plan_taps(&assignments, OWN);
        assert_eq!(taps.len(), 3);
        assert_eq!(taps[0].source, TapSource::Include(vec![100]));
        assert_eq!(taps[0].channel, ChannelId::Game);
        assert_eq!(taps[1].source, TapSource::Include(vec![200, 201]));
        assert_eq!(
            taps[2].source,
            TapSource::AllExcept(vec![OWN, 100, 200, 201]),
            "explicit Media apps stay in the catch-all tap"
        );
    }

    #[test]
    fn own_process_is_never_tapped() {
        let assignments = BTreeMap::from([(OWN, ChannelId::Game)]);
        let taps = plan_taps(&assignments, OWN);
        assert_eq!(taps.len(), 1);
        assert_eq!(taps[0].channel, ChannelId::Media);
    }
}
