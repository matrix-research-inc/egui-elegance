//! The [`IdSalt`] bound shared by every widget that takes a caller-supplied
//! id salt.

use std::fmt::Debug;
use std::hash::Hash;

/// A value usable as a widget *id salt*.
///
/// Many elegance widgets persist state across frames (open/closed, focus,
/// drag order) and therefore need a stable, unique [`egui::Id`]. Callers
/// supply a *salt* — a string, integer, tuple, or any other hashable value —
/// which the widget hashes together with its location into that id.
///
/// egui builds an id from a salt with [`egui::Id::unique`], which requires the
/// source to be both [`Hash`] (to derive the id) and [`Debug`] (so egui can
/// name the source when it reports an id collision in debug builds).
/// `IdSalt` bundles exactly that bound, so every widget can spell its
/// parameter `impl IdSalt` and the requirement lives in one place: if egui
/// ever changes what an id source must implement, only this declaration
/// changes.
///
/// You never implement `IdSalt` by hand — a blanket impl covers every
/// `Hash + Debug` type.
///
/// This is a *bound on salt sources*, distinct from egui's own
/// [`egui::IdSalt`], which is the concrete hashed-salt value.
pub trait IdSalt: Hash + Debug {}

impl<T: Hash + Debug> IdSalt for T {}
