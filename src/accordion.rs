//! Accordion — a vertical stack of collapsible items inside a single bordered
//! panel. Each row has a chevron, title, optional subtitle, optional icon
//! halo, and an optional right-aligned meta slot. Click a header (or press
//! Space/Enter when focused) to toggle its panel; in `exclusive` mode opening
//! one item closes any other.
//!
//! ```no_run
//! # use elegance::Accordion;
//! # egui::__run_test_ui(|ui| {
//! Accordion::new("faq").show(ui, |acc| {
//!     acc.item("How do I invite teammates?")
//!         .default_open(true)
//!         .show(|ui| { ui.label("Open Settings ▸ Members."); });
//!     acc.item("Archive a project")
//!         .show(|ui| { ui.label("Hidden from sidebar."); });
//! });
//! # });
//! ```

use egui::{
    Align2, Color32, CornerRadius, FontId, Frame, Id, InnerResponse, Key, Margin, Pos2, Rect, Role,
    Sense, Shape, Stroke, StrokeKind, Ui, Vec2, WidgetInfo, WidgetText, layers::ShapeIdx, pos2,
};

use crate::Accent;
use crate::theme::{Theme, with_alpha};

/// Boxed `FnOnce(&mut Ui)` callback used by the meta slot.
type UiFn<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

const HEADER_PAD_X: f32 = 16.0;
const HEADER_PAD_Y: f32 = 13.0;
const FLUSH_HEADER_PAD_Y: f32 = 12.0;
const CHEVRON_SIZE: f32 = 12.0;
const CHEVRON_GAP: f32 = 12.0;
const ICON_SIZE: f32 = 26.0;
const ICON_GAP: f32 = 10.0;
const META_GAP: f32 = 10.0;

/// A grouped stack of collapsible items inside one bordered panel.
///
/// Use [`Accordion::new`] to create one, then call [`Accordion::show`] with
/// a closure that declares the items via the [`AccordionUi`] handle.
#[must_use = "Call `.show(ui, |acc| ...)` to render the accordion."]
pub struct Accordion {
    id_salt: Id,
    exclusive: bool,
    flush: bool,
}

impl std::fmt::Debug for Accordion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Accordion")
            .field("id_salt", &self.id_salt)
            .field("exclusive", &self.exclusive)
            .field("flush", &self.flush)
            .finish()
    }
}

impl Accordion {
    /// Create an accordion keyed by `id_salt`. Per-item open state lives in
    /// egui temp storage scoped under this salt, so different accordions on
    /// the same page must use distinct salts.
    pub fn new(id_salt: impl crate::IdSalt) -> Self {
        Self {
            id_salt: Id::unique(("elegance_accordion", id_salt)),
            exclusive: false,
            flush: false,
        }
    }

    /// Allow only one item to be open at a time. Opening another item closes
    /// the current one. Default: `false` (any number of items may be open).
    #[inline]
    pub fn exclusive(mut self, exclusive: bool) -> Self {
        self.exclusive = exclusive;
        self
    }

    /// Drop the outer border and use only thin dividers between rows.
    /// Useful when the accordion sits inside an existing surface that
    /// already provides chrome. Default: `false`.
    #[inline]
    pub fn flush(mut self, flush: bool) -> Self {
        self.flush = flush;
        self
    }

    /// Render the accordion. The closure receives an [`AccordionUi`] handle
    /// used to declare each item.
    pub fn show<R>(self, ui: &mut Ui, body: impl FnOnce(&mut AccordionUi<'_>) -> R) -> R {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;
        let radius = CornerRadius::same(theme.card_radius as u8);

        let exclusive_id = self.id_salt.with("__exclusive_open");
        let seeded_id = self.id_salt.with("__seeded");
        let prev_open: Option<usize> = ui.ctx().data(|d| d.get_temp(exclusive_id));
        let already_seeded: bool = ui
            .ctx()
            .data(|d| d.get_temp::<bool>(seeded_id).unwrap_or(false));

        let frame = if self.flush {
            Frame::new()
        } else {
            Frame::new()
                .fill(p.card)
                .stroke(Stroke::new(1.0, p.border))
                .corner_radius(radius)
        };

        frame
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let mut handle = AccordionUi {
                    ui,
                    id_salt: self.id_salt,
                    exclusive: self.exclusive,
                    flush: self.flush,
                    next_index: 0,
                    item_count: 0,
                    highlights: Vec::new(),
                    prev_open_exclusive: prev_open,
                    next_open_exclusive: prev_open,
                    seeded: already_seeded,
                    focus_chain: Vec::new(),
                };
                let r = body(&mut handle);

                // Now that every item is declared, paint the row highlights
                // that were deferred so they could round the card's corners.
                handle.flush_highlights();

                let final_open = handle.next_open_exclusive;
                let any_items = handle.next_index > 0;
                let chain = std::mem::take(&mut handle.focus_chain);
                let ctx = handle.ui.ctx().clone();

                if self.exclusive && final_open != prev_open {
                    ctx.data_mut(|d| match final_open {
                        Some(idx) => {
                            d.insert_temp(exclusive_id, idx);
                        }
                        None => {
                            d.remove::<usize>(exclusive_id);
                        }
                    });
                }
                // Mark defaults consumed once the first frame has rendered
                // any items, so default_open never re-fires after the user
                // has had a chance to make an explicit choice.
                if !already_seeded && any_items {
                    ctx.data_mut(|d| d.insert_temp(seeded_id, true));
                }
                if !chain.is_empty() {
                    handle_focus_chain_keys(&ctx, &chain);
                }
                r
            })
            .inner
    }
}

/// Handle passed to the [`Accordion::show`] body closure for declaring items.
///
/// Each call to [`AccordionUi::item`] returns an [`AccordionItem`] builder
/// that you finish with [`AccordionItem::show`] to render the row plus body.
pub struct AccordionUi<'u> {
    ui: &'u mut Ui,
    id_salt: Id,
    exclusive: bool,
    flush: bool,
    next_index: usize,
    item_count: usize,
    /// Hover/focus row highlights deferred until the body closure finishes.
    /// A highlight needs to know whether its row is the *last* one (to round
    /// the card's bottom corners), which isn't known until every item has been
    /// declared. So each highlighted row reserves a shape slot at its paint
    /// position and records itself here; [`AccordionUi::flush_highlights`]
    /// fills those slots once the final count is in hand.
    highlights: Vec<PendingHighlight>,
    prev_open_exclusive: Option<usize>,
    next_open_exclusive: Option<usize>,
    seeded: bool,
    focus_chain: Vec<Id>,
}

/// A hover/focus highlight whose painting is deferred (see
/// [`AccordionUi::highlights`]). `slot` was reserved behind the row's content,
/// so filling it later keeps the highlight under the chevron/title.
struct PendingHighlight {
    slot: ShapeIdx,
    rect: Rect,
    index: usize,
    is_open: bool,
    focused: bool,
}

impl<'u> std::fmt::Debug for AccordionUi<'u> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccordionUi")
            .field("id_salt", &self.id_salt)
            .field("exclusive", &self.exclusive)
            .field("flush", &self.flush)
            .field("next_index", &self.next_index)
            .field("prev_open_exclusive", &self.prev_open_exclusive)
            .finish()
    }
}

impl<'u> AccordionUi<'u> {
    /// Begin a new item with the given title. Finish with
    /// [`AccordionItem::show`].
    pub fn item(&mut self, title: impl Into<WidgetText>) -> AccordionItem<'_, 'u> {
        let index = self.next_index;
        self.next_index += 1;
        AccordionItem {
            owner: self,
            index,
            title: title.into(),
            subtitle: None,
            icon: None,
            accent: None,
            meta: None,
            default_open: false,
            disabled: false,
        }
    }

    /// Paint the deferred row highlights. Called once the body closure has
    /// declared every item, so `next_index` is the true count and the last
    /// row is known. Each highlight fills the shape slot it reserved at paint
    /// time, which keeps it behind the row content. The corners that meet the
    /// card's rounded corners (top on the first row, bottom on the last closed
    /// row) are rounded to match; the rest stay square. See issue #7.
    fn flush_highlights(&mut self) {
        if self.highlights.is_empty() {
            return;
        }
        let theme = Theme::current(self.ui.ctx());
        let p = &theme.palette;
        let total = self.next_index;
        let card_r = if self.flush {
            0
        } else {
            theme.card_radius as u8
        };
        for h in std::mem::take(&mut self.highlights) {
            let is_first = h.index == 0;
            // A closed final row abuts the card's rounded bottom corners; an
            // open one puts its (fill-less) body there, so its header is
            // interior and stays square.
            let is_last = !h.is_open && h.index + 1 == total;
            let corners = |r: u8| CornerRadius {
                nw: if is_first { r } else { 0 },
                ne: if is_first { r } else { 0 },
                sw: if is_last { r } else { 0 },
                se: if is_last { r } else { 0 },
            };
            let alpha = if h.focused { 12 } else { 8 };
            let lift = Color32::from_rgba_unmultiplied(p.text.r(), p.text.g(), p.text.b(), alpha);
            let mut shapes = vec![Shape::rect_filled(h.rect, corners(card_r), lift)];
            if h.focused {
                // The ring sits 1px inside the row, so its outer corners curve
                // one pixel tighter than the card to stay concentric with it.
                shapes.push(Shape::rect_stroke(
                    h.rect.shrink(1.0),
                    corners(card_r.saturating_sub(1)),
                    Stroke::new(2.0, with_alpha(p.focus, 180)),
                    StrokeKind::Inside,
                ));
            }
            self.ui.painter().set(h.slot, Shape::Vec(shapes));
        }
    }
}

/// A single row in an [`Accordion`]. Configure with the builder methods and
/// finish with [`AccordionItem::show`].
#[must_use = "Call `.show(|ui| ...)` to render the item."]
pub struct AccordionItem<'a, 'u> {
    owner: &'a mut AccordionUi<'u>,
    index: usize,
    title: WidgetText,
    subtitle: Option<WidgetText>,
    icon: Option<WidgetText>,
    accent: Option<Accent>,
    meta: Option<UiFn<'a>>,
    default_open: bool,
    disabled: bool,
}

impl<'a, 'u> std::fmt::Debug for AccordionItem<'a, 'u> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccordionItem")
            .field("index", &self.index)
            .field("title", &self.title.text())
            .field("subtitle", &self.subtitle.as_ref().map(|s| s.text()))
            .field("icon", &self.icon.as_ref().map(|s| s.text()))
            .field("accent", &self.accent)
            .field("default_open", &self.default_open)
            .field("disabled", &self.disabled)
            .finish()
    }
}

impl<'a, 'u> AccordionItem<'a, 'u> {
    /// Set a muted subtitle line shown beneath the title.
    #[inline]
    pub fn subtitle(mut self, subtitle: impl Into<WidgetText>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Show a small tinted icon halo to the left of the title. Pass any
    /// short glyph (e.g. an emoji or a symbol from the bundled
    /// `Elegance Symbols` font).
    #[inline]
    pub fn icon(mut self, icon: impl Into<WidgetText>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Override the icon halo accent. No-op without [`AccordionItem::icon`].
    /// Default: a neutral muted treatment.
    #[inline]
    pub fn accent(mut self, accent: Accent) -> Self {
        self.accent = Some(accent);
        self
    }

    /// Render arbitrary widgets in the right-aligned meta slot of the
    /// header row (badges, status dots, counts).
    #[inline]
    pub fn meta<F: FnOnce(&mut Ui) + 'a>(mut self, add_meta: F) -> Self {
        self.meta = Some(Box::new(add_meta));
        self
    }

    /// Open this item by default the first time it is rendered. After that,
    /// the user's choice is remembered.
    #[inline]
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Render the item as disabled — the row dims and clicks/keys are
    /// ignored. Useful for permission-gated items.
    #[inline]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Render the header row, and the body when open. Returns an
    /// [`InnerResponse`] whose `response` is the header row and whose
    /// `inner` is the body closure's return value (or `None` when closed).
    pub fn show<R>(self, add_body: impl FnOnce(&mut Ui) -> R) -> InnerResponse<Option<R>> {
        let AccordionItem {
            owner,
            index,
            title,
            subtitle,
            icon,
            accent,
            meta,
            default_open,
            disabled,
        } = self;

        let theme = Theme::current(owner.ui.ctx());
        let p = &theme.palette;
        let t = &theme.typography;

        // Resolve the open state from storage.
        let item_id = owner.id_salt.with(("item", index));
        let mut is_open = if owner.exclusive {
            owner.next_open_exclusive == Some(index)
        } else {
            owner
                .ui
                .ctx()
                .data(|d| d.get_temp::<bool>(item_id).unwrap_or(default_open))
        };

        // First-frame defaults. In exclusive mode we honour the *first*
        // item that asks for `default_open(true)` and ignore the rest.
        if !owner.seeded && default_open && !is_open {
            if owner.exclusive {
                if owner.next_open_exclusive.is_none() {
                    is_open = true;
                    owner.next_open_exclusive = Some(index);
                }
            } else {
                is_open = true;
            }
        }

        // Divider above every item except the first.
        if owner.item_count > 0 {
            let avail = owner.ui.available_rect_before_wrap();
            let y = avail.min.y + 0.5;
            owner.ui.painter().line_segment(
                [pos2(avail.min.x, y), pos2(avail.max.x, y)],
                Stroke::new(1.0, p.border),
            );
        }
        owner.item_count += 1;

        // --- Header row --------------------------------------------------
        let pad_y = if owner.flush {
            FLUSH_HEADER_PAD_Y
        } else {
            HEADER_PAD_Y
        };
        let pad_x = if owner.flush { 0.0 } else { HEADER_PAD_X };

        let title_galley =
            crate::theme::placeholder_galley(owner.ui, title.text(), t.body, true, f32::INFINITY);
        let subtitle_galley = subtitle.as_ref().map(|s| {
            crate::theme::placeholder_galley(owner.ui, s.text(), t.small, false, f32::INFINITY)
        });

        let icon_block_w = if icon.is_some() {
            ICON_SIZE + ICON_GAP
        } else {
            0.0
        };
        let title_block_h = match &subtitle_galley {
            Some(s) => title_galley.size().y + 2.0 + s.size().y,
            None => title_galley.size().y,
        };
        let row_content_h = title_block_h.max(ICON_SIZE).max(CHEVRON_SIZE);
        let row_h = row_content_h + pad_y * 2.0;

        let avail_w = owner.ui.available_size_before_wrap().x;
        let (row_rect, row_resp) = owner.ui.allocate_exact_size(
            Vec2::new(avail_w, row_h),
            if disabled {
                Sense::hover()
            } else {
                Sense::click()
            },
        );

        if !disabled {
            owner.focus_chain.push(row_resp.id);
        }

        if owner.ui.is_rect_visible(row_rect) {
            // Defer the hover/focus highlight: reserve a slot now (so it paints
            // behind this row's content) and record the row. Once the body
            // closure has declared every item, `flush_highlights` fills the
            // slot, rounding the corners that meet the card (top on the first
            // row, bottom on the last) — a square fill/ring would otherwise
            // square off the card's curve (issue #7's pattern).
            if !disabled && (row_resp.hovered() || row_resp.has_focus()) {
                let slot = owner.ui.painter().add(Shape::Noop);
                owner.highlights.push(PendingHighlight {
                    slot,
                    rect: row_rect,
                    index,
                    is_open,
                    focused: row_resp.has_focus(),
                });
            }

            let dim = if disabled { 0.55 } else { 1.0 };
            let mut x = row_rect.min.x + pad_x;
            let cy = row_rect.center().y;

            let chev_color = if !disabled && row_resp.hovered() {
                p.text_muted
            } else {
                p.text_faint
            };
            paint_chevron(
                owner.ui,
                pos2(x + CHEVRON_SIZE * 0.5, cy),
                CHEVRON_SIZE,
                fade(chev_color, dim),
                is_open,
            );
            x += CHEVRON_SIZE + CHEVRON_GAP;

            if let Some(icon_text) = icon.as_ref() {
                let icon_rect = egui::Rect::from_center_size(
                    pos2(x + ICON_SIZE * 0.5, cy),
                    Vec2::splat(ICON_SIZE),
                );
                paint_icon_square(owner.ui, icon_rect, icon_text.text(), accent, &theme, dim);
                x += icon_block_w;
            }

            let title_color = fade(p.text, dim);
            let title_y = match &subtitle_galley {
                Some(_) => cy - title_block_h * 0.5,
                None => cy - title_galley.size().y * 0.5,
            };
            owner
                .ui
                .painter()
                .galley(pos2(x, title_y), title_galley.clone(), title_color);
            if let Some(sub) = &subtitle_galley {
                let sub_y = title_y + title_galley.size().y + 2.0;
                owner
                    .ui
                    .painter()
                    .galley(pos2(x, sub_y), sub.clone(), fade(p.text_faint, dim));
            }

            if let Some(add_meta) = meta {
                let meta_pad_x = pad_x.max(META_GAP);
                let meta_max_w = (row_rect.width() * 0.5).max(60.0);
                let meta_rect = egui::Rect::from_min_max(
                    pos2(row_rect.max.x - meta_max_w - meta_pad_x, row_rect.min.y),
                    pos2(row_rect.max.x - meta_pad_x, row_rect.max.y),
                );
                let mut meta_ui = owner.ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(meta_rect)
                        .layout(egui::Layout::right_to_left(egui::Align::Center)),
                );
                if disabled {
                    meta_ui.disable();
                }
                add_meta(&mut meta_ui);
            }
        }

        // --- Click / keyboard --------------------------------------------
        let mut toggle = false;
        if !disabled {
            if row_resp.clicked() {
                toggle = true;
            }
            if row_resp.has_focus() {
                let pressed = owner
                    .ui
                    .ctx()
                    .input(|i| i.key_pressed(Key::Space) || i.key_pressed(Key::Enter));
                if pressed {
                    toggle = true;
                }
            }
        }
        if toggle {
            is_open = !is_open;
        }

        // Persist.
        if owner.exclusive {
            owner.next_open_exclusive = if is_open {
                Some(index)
            } else if owner.next_open_exclusive == Some(index) {
                None
            } else {
                owner.next_open_exclusive
            };
        } else {
            owner.ui.ctx().data_mut(|d| d.insert_temp(item_id, is_open));
        }

        // Accessibility.
        let title_text = title.text().to_string();
        row_resp.widget_info(|| {
            WidgetInfo::selected(Role::DisclosureTriangle, !disabled, is_open, &title_text)
        });

        // --- Body --------------------------------------------------------
        let inner = if is_open {
            let body_pad_left = if owner.flush {
                CHEVRON_SIZE + CHEVRON_GAP
            } else {
                pad_x + CHEVRON_SIZE + CHEVRON_GAP
            };
            let body_pad_right = pad_x.max(HEADER_PAD_X);
            let body_pad_top = if owner.flush { 4.0 } else { 6.0 };
            let body_pad_bottom = if owner.flush { 14.0 } else { 16.0 };
            let body_frame = Frame::new().inner_margin(Margin {
                left: body_pad_left as i8,
                right: body_pad_right as i8,
                top: body_pad_top as i8,
                bottom: body_pad_bottom as i8,
            });
            let r = body_frame.show(owner.ui, |ui| add_body(ui)).inner;
            Some(r)
        } else {
            None
        };

        InnerResponse::new(inner, row_resp)
    }
}

fn handle_focus_chain_keys(ctx: &egui::Context, chain: &[Id]) {
    let Some(focused) = ctx.memory(|m| m.focused()) else {
        return;
    };
    let Some(idx) = chain.iter().position(|id| *id == focused) else {
        return;
    };

    let (down, up, home, end) = ctx.input(|i| {
        (
            i.key_pressed(Key::ArrowDown),
            i.key_pressed(Key::ArrowUp),
            i.key_pressed(Key::Home),
            i.key_pressed(Key::End),
        )
    });
    let target = if down && idx + 1 < chain.len() {
        Some(chain[idx + 1])
    } else if up && idx > 0 {
        Some(chain[idx - 1])
    } else if home {
        chain.first().copied()
    } else if end {
        chain.last().copied()
    } else {
        None
    };
    if let Some(target) = target {
        ctx.memory_mut(|m| m.request_focus(target));
    }
}

fn fade(c: Color32, t: f32) -> Color32 {
    let a = (c.a() as f32 * t).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

fn paint_chevron(ui: &Ui, center: Pos2, size: f32, color: Color32, open: bool) {
    let half = size * 0.32;
    let points: Vec<Pos2> = if open {
        vec![
            pos2(center.x - half, center.y - half * 0.55),
            pos2(center.x + half, center.y - half * 0.55),
            pos2(center.x, center.y + half * 0.75),
        ]
    } else {
        vec![
            pos2(center.x - half * 0.55, center.y - half),
            pos2(center.x - half * 0.55, center.y + half),
            pos2(center.x + half * 0.75, center.y),
        ]
    };
    ui.painter()
        .add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
}

fn paint_icon_square(
    ui: &Ui,
    rect: egui::Rect,
    glyph: &str,
    accent: Option<Accent>,
    theme: &Theme,
    dim: f32,
) {
    let p = &theme.palette;
    let (bg, stroke, fg) = match accent {
        Some(a) => {
            let base = p.accent_fill(a);
            (
                with_alpha(base, 28),
                Stroke::new(1.0, with_alpha(base, 90)),
                base,
            )
        }
        None => (
            p.depth_tint(p.card, 0.05),
            Stroke::new(1.0, p.border),
            p.text_muted,
        ),
    };
    ui.painter()
        .rect(rect, CornerRadius::same(6), bg, stroke, StrokeKind::Inside);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        glyph,
        FontId::proportional(theme.typography.label),
        fade(fg, dim),
    );
}
