import tkinter as tk
from tkinter import messagebox, ttk
from typing import Any, Callable, Optional

import cache
import sql_builder
import units
from models import UniverseState
from ui.output_panel import OutputPanel
from ui.widgets_common import UnitEntry
from validation import validate_universe_state


class FrameUniverseState(ttk.Frame):
    def __init__(
        self,
        parent: tk.Widget,
        output_panel: OutputPanel,
        on_cache_updated: Optional[Callable[[], None]] = None,
        **kwargs: Any,
    ) -> None:
        super().__init__(parent, padding=12, **kwargs)

        self.output_panel = output_panel
        self.on_cache_updated = on_cache_updated

        self.columnconfigure(0, weight=1)

        state_frame = ttk.LabelFrame(self, text="Estado Global da Época", padding=10)
        state_frame.grid(row=0, column=0, sticky="ew", padx=6, pady=6)
        state_frame.columnconfigure(0, weight=1)

        self.entry_elapsed = UnitEntry(
            state_frame,
            "Tempo decorrido (J2000):",
            [
                ("anos", units.julian_years_to_seconds),
                ("dias", units.days_to_seconds),
                ("segundos", lambda x: x),
            ],
            default_unit_idx=0,
            initial_value="0.0",
        )
        self.entry_elapsed.grid(row=0, column=0, sticky="w", pady=4)

        actions_frame = ttk.Frame(self)
        actions_frame.grid(row=1, column=0, sticky="ew", padx=6, pady=12)

        ttk.Button(
            actions_frame,
            text="Gerar SQL",
            command=self.generate_sql,
        ).pack(side=tk.LEFT, padx=(0, 6))

        ttk.Button(
            actions_frame,
            text="Limpar",
            command=self.clear_form,
        ).pack(side=tk.LEFT)

    def build_model(self) -> UniverseState:
        val = self.entry_elapsed.get_si_value()
        return UniverseState(
            id=1,
            seconds_since_j2000_epoch=val if val is not None else 0.0,
        )

    def generate_sql(self) -> Optional[UniverseState]:
        model = self.build_model()
        errors = validate_universe_state(model)
        if errors:
            messagebox.showerror("Erro de Validação", "\n".join(f"• {e}" for e in errors))
            return None

        sql = sql_builder.build_insert_universe_state(model, atomic=True)
        self.output_panel.append_sql(sql)
        return model

    def clear_form(self) -> None:
        self.entry_elapsed.set_value("0.0", unit="anos")
