use crate::theme::*;
use crate::view::MainView;
use gametracker_core::db::Rule;
use gametracker_core::detect;
use gametracker_core::keymap::Layout;
use gametracker_core::{Settings, autostart, transfer};
use gpui::{Context, PathPromptOptions, SharedString, Window, div, prelude::*, px, rgb};
use gpui_elements::editable_text::text_input;
use std::path::PathBuf;

impl MainView {
    fn change_settings(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut Settings)) {
        if let Err(e) = self.shared.update_settings(f) {
            self.flash(
                false,
                format!("Einstellungen konnten nicht gespeichert werden: {e:#}"),
                cx,
            );
        }
        self.reload(cx);
    }

    fn set_rule(&mut self, rule: Rule, msg: String, cx: &mut Context<Self>) {
        if let Some(db) = &self.db {
            match db.set_rule(&rule) {
                Ok(()) => {
                    self.shared.rules_changed();
                    self.flash(true, msg, cx);
                }
                Err(e) => self.flash(false, format!("{e:#}"), cx),
            }
        }
        self.reload(cx);
    }

    fn export(&mut self, cx: &mut Context<Self>) {
        let dir = dirs::document_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."));
        let rx = cx.prompt_for_new_path(&dir, Some(&transfer::default_export_name()));
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(path))) = rx.await else { return };
            this.update(cx, |this, cx| {
                let Some(db) = &this.db else { return };
                match transfer::export_to(db, &path) {
                    Ok(()) => this.flash(true, format!("Exportiert nach {}", path.display()), cx),
                    Err(e) => this.flash(false, format!("Export fehlgeschlagen: {e:#}"), cx),
                }
            })
            .ok();
        })
        .detach();
    }

    fn import(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Importieren".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else {
                return;
            };
            this.update(cx, |this, cx| {
                let Some(db) = &mut this.db else { return };
                let mut msgs = Vec::new();
                let mut ok = true;
                for path in &paths {
                    match transfer::import_from(db, path) {
                        Ok(sum) => msgs.push(format!("Import erfolgreich: {sum}.")),
                        Err(e) => {
                            ok = false;
                            msgs.push(format!(
                                "Import von {} fehlgeschlagen: {e:#}",
                                path.display()
                            ));
                        }
                    }
                }
                this.shared.rules_changed();
                this.flash(ok, msgs.join(" "), cx);
                this.reload(cx);
            })
            .ok();
        })
        .detach();
    }

    fn add_exe(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Als Spiel hinzufügen".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else { return };
            let Some(path) = paths.into_iter().next() else { return };
            this.update(cx, |this, cx| {
                let Some(exe) = detect::exe_name(&path) else { return };
                let name = detect::name_for_exe_path(&path);
                this.set_rule(
                    Rule { exe: exe.clone(), name: name.clone(), ignored: false },
                    format!("„{name}“ ({exe}) wird jetzt getrackt. Umbenennen kannst du es unter Spiele."),
                    cx,
                );
            })
            .ok();
        })
        .detach();
    }

    pub fn render_settings(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let s = self.settings.clone();

        let tracking = card()
            .flex()
            .flex_col()
            .gap_1()
            .child(section_title("TRACKING"))
            .child(
                toggle("t-games", "Spiele tracken", "Erkennt laufende Spiele und speichert Spielzeiten.", s.track_games)
                    .on_click(cx.listener(|this, _, _, cx| this.change_settings(cx, |s| s.track_games = !s.track_games))),
            )
            .child(
                toggle(
                    "t-keys",
                    "Tastendrücke zählen",
                    "Zählt nur, wie oft jede Taste pro Tag gedrückt wird – kein Text, keine Reihenfolge.",
                    s.track_keys,
                )
                .on_click(cx.listener(|this, _, _, cx| this.change_settings(cx, |s| s.track_keys = !s.track_keys))),
            )
            .child(
                toggle(
                    "t-auto",
                    "Spiele automatisch erkennen",
                    "Steam-, Epic-, GOG-, Xbox-, Ubisoft-, EA-Ordner und bekannte Spiele. Ohne: nur deine eigenen Regeln.",
                    s.auto_detect,
                )
                .on_click(cx.listener(|this, _, _, cx| this.change_settings(cx, |s| s.auto_detect = !s.auto_detect))),
            )
            .child(
                toggle(
                    "t-bg",
                    "Beim Schließen im Hintergrund weiterlaufen",
                    "Fenster schließen stoppt das Tracking nicht. Zum Öffnen GameTracker einfach erneut starten.",
                    s.run_in_background,
                )
                .on_click(
                    cx.listener(|this, _, _, cx| this.change_settings(cx, |s| s.run_in_background = !s.run_in_background)),
                ),
            )
            .child(
                toggle("t-autostart", "Mit dem System starten", "Startet unsichtbar im Hintergrund beim Anmelden.", self.autostart)
                    .on_click(cx.listener(|this, _, _, cx| {
                        let want = !this.autostart;
                        match autostart::set_enabled(want) {
                            Ok(()) => this.autostart = autostart::is_enabled(),
                            Err(e) => this.flash(false, format!("{e:#}"), cx),
                        }
                        cx.notify();
                    })),
            );

        let layout =
            card()
                .flex()
                .flex_col()
                .gap_3()
                .child(section_title("TASTATUR-LAYOUT"))
                .child(muted(
                    "Nur für die Beschriftung der Heatmap. Gezählt wird immer die physische Taste.",
                ))
                .child(div().flex().gap_1().children(
                    [Layout::Qwertz, Layout::Qwerty].into_iter().map(|l| {
                        chip(
                            SharedString::from(format!("layout-{l:?}")),
                            l.label(),
                            s.layout == l,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.change_settings(cx, |s| s.layout = l)
                        }))
                    }),
                ));

        let backup_options: [(u32, &str); 4] = [
            (0, "Aus"),
            (1, "Täglich"),
            (7, "Wöchentlich"),
            (30, "Monatlich"),
        ];
        let data = card()
            .flex()
            .flex_col()
            .gap_3()
            .child(section_title("DATEN, EXPORT & IMPORT"))
            .child(muted(
                "Alles wird lokal gespeichert. Für einen PC-Wechsel: hier exportieren, auf dem neuen PC importieren. \
                 Importieren ist sicher wiederholbar – nichts wird doppelt gezählt.",
            ))
            .child(div().text_xs().text_color(rgb(FAINT)).child(format!("Datenbank: {}", self.shared.db_path.display())))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(primary_button("export", "Exportieren …").on_click(cx.listener(|this, _, _, cx| this.export(cx))))
                    .child(button("import", "Importieren …").on_click(cx.listener(|this, _, _, cx| this.import(cx))))
                    .child(button("open-backups", "Backup-Ordner öffnen").on_click(cx.listener(|this, _, _, cx| {
                        let dir = this.shared.backup_dir.clone();
                        std::fs::create_dir_all(&dir).ok();
                        cx.open_with_system(&dir);
                    }))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().text_sm().child("Automatisches Backup:"))
                    .child(div().flex().gap_1().children(backup_options.into_iter().map(|(days, label)| {
                        chip(SharedString::from(format!("backup-{days}")), label, s.backup_every_days == days)
                            .on_click(cx.listener(move |this, _, _, cx| this.change_settings(cx, |s| s.backup_every_days = days)))
                    }))),
            );

        let rules = card()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(section_title("EIGENE REGELN"))
                    .child(button("add-exe", "Spiel-EXE auswählen …").on_click(cx.listener(|this, _, _, cx| this.add_exe(cx)))),
            )
            .child(muted("Wird ein Spiel nicht erkannt, füge seine EXE hinzu. Fälschlich erkannte Programme kannst du ignorieren."))
            .when(self.rules.is_empty(), |d| d.child(div().text_sm().text_color(rgb(FAINT)).child("Noch keine Regeln.")))
            .children(self.rules.iter().enumerate().map(|(i, r)| {
                let exe = r.exe.clone();
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .py_1()
                    .text_sm()
                    .child(
                        div()
                            .px_2()
                            .rounded_sm()
                            .text_xs()
                            .when(r.ignored, |d| d.bg(rgb(0x3a2a14)).text_color(rgb(WARN)).child("ignoriert"))
                            .when(!r.ignored, |d| d.bg(rgb(0x123a30)).text_color(rgb(LIVE)).child("Spiel")),
                    )
                    .child(div().w(px(240.)).truncate().child(if r.ignored { r.exe.clone() } else { r.name.clone() }))
                    .child(div().flex_1().truncate().text_color(rgb(MUTED)).child(if r.ignored { String::new() } else { r.exe.clone() }))
                    .child(button(("rm-rule", i), "Entfernen").on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(db) = &this.db {
                            db.remove_rule(&exe).ok();
                            this.shared.rules_changed();
                        }
                        this.reload(cx);
                    })))
            }));

        // Running programs
        let filter = self.proc_filter.read(cx).as_str().trim().to_lowercase();
        let procs: Vec<_> = self
            .running
            .iter()
            .filter(|p| p.path.is_some())
            .filter(|p| {
                filter.is_empty()
                    || p.exe.contains(&filter)
                    || p.path
                        .as_deref()
                        .is_some_and(|x| x.to_lowercase().contains(&filter))
                    || p.detected_as
                        .as_deref()
                        .is_some_and(|x| x.to_lowercase().contains(&filter))
            })
            .collect();
        let mut procs = procs;
        procs.sort_by_key(|p| (p.detected_as.is_none(), p.exe.clone()));
        let shown = 40;
        let running = card()
            .flex()
            .flex_col()
            .gap_3()
            .child(section_title("LAUFENDE PROGRAMME"))
            .child(muted(
                "Starte dein Spiel und markiere es hier, falls es nicht automatisch erkannt wird.",
            ))
            .child(
                text_input("proc-filter")
                    .state(self.proc_filter.downgrade())
                    .placeholder("Suchen …")
                    .w(px(320.))
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .bg(rgb(BG))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .text_sm()
                    .whitespace_nowrap(),
            )
            .children(procs.iter().take(shown).enumerate().map(|(i, p)| {
                let exe = p.exe.clone();
                let path = p.path.clone().unwrap_or_default();
                let detected = p.detected_as.clone();
                let action = match &detected {
                    Some(name) => {
                        let name = name.clone();
                        button(("ignore-proc", i), "Ignorieren").on_click(cx.listener(
                            move |this, _, _, cx| {
                                this.set_rule(
                                    Rule {
                                        exe: exe.clone(),
                                        name: String::new(),
                                        ignored: true,
                                    },
                                    format!("{exe} („{name}“) wird ab jetzt ignoriert."),
                                    cx,
                                );
                            },
                        ))
                    }
                    None => button(("track-proc", i), "Als Spiel tracken").on_click(cx.listener(
                        move |this, _, _, cx| {
                            let name = detect::name_for_exe_path(std::path::Path::new(&path));
                            this.set_rule(
                                Rule {
                                    exe: exe.clone(),
                                    name: name.clone(),
                                    ignored: false,
                                },
                                format!("„{name}“ wird jetzt getrackt."),
                                cx,
                            );
                        },
                    )),
                };
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .py_1()
                    .text_sm()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .child(div().child(p.exe.clone()))
                                    .when_some(detected.clone(), |d, n| {
                                        d.child(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(LIVE))
                                                .child(format!("● erkannt als {n}")),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(FAINT))
                                    .truncate()
                                    .child(p.path.clone().unwrap_or_default()),
                            ),
                    )
                    .child(action)
            }))
            .when(procs.len() > shown, |d| {
                d.child(div().text_xs().text_color(rgb(FAINT)).child(format!(
                    "… {} weitere – nutze die Suche.",
                    procs.len() - shown
                )))
            });

        let message = self.message.clone().map(|(ok, msg)| {
            div()
                .p_3()
                .rounded_md()
                .text_sm()
                .border_1()
                .when(ok, |d| d.bg(rgb(0x123a30)).border_color(rgb(LIVE)))
                .when(!ok, |d| d.bg(rgb(0x3a1d22)).border_color(rgb(DANGER)))
                .child(msg)
        });

        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(title("Einstellungen"))
            .children(message)
            .when_some(self.key_error.clone(), |d, e| {
                d.child(
                    card()
                        .border_color(rgb(WARN))
                        .child(div().text_sm().text_color(rgb(WARN)).child(e)),
                )
            })
            .child(tracking)
            .child(layout)
            .child(data)
            .child(rules)
            .child(running)
    }
}
