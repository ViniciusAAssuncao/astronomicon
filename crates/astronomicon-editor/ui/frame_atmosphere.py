import tkinter as tk
from tkinter import messagebox, ttk
from typing import Any, Callable, Dict, List, Optional, Tuple

import cache
import elements
import sql_builder
import units
from models import Atmosphere, AtmosphereGasComponent
from ui.output_panel import OutputPanel
from ui.widgets_common import UnitEntry
from validation import validate_atmosphere


class GasComponentRow(ttk.Frame):
    def __init__(
        self,
        parent: tk.Widget,
        on_remove: Callable[["GasComponentRow"], None],
        on_changed: Callable[[], None],
        initial_formula: str = "",
        initial_percentage: str = "",
        **kwargs: Any,
    ) -> None:
        super().__init__(parent, **kwargs)

        self.on_remove = on_remove
        self.on_changed = on_changed

        self.formula_var = tk.StringVar(value=initial_formula)
        self.formula_var.trace_add("write", self._on_formula_write)

        self.formula_entry = ttk.Entry(self, textvariable=self.formula_var, width=12)
        self.formula_entry.pack(side=tk.LEFT, padx=(0, 6))

        self.formula_status = ttk.Label(self, text="OK", foreground="#2E7D32", width=6)
        self.formula_status.pack(side=tk.LEFT, padx=(0, 6))

        self.percentage_var = tk.StringVar(value=initial_percentage)
        self.percentage_var.trace_add("write", self._on_percentage_write)

        self.percentage_entry = ttk.Entry(self, textvariable=self.percentage_var, width=10)
        self.percentage_entry.pack(side=tk.LEFT, padx=(0, 4))

        ttk.Label(self, text="%").pack(side=tk.LEFT, padx=(0, 8))

        self.remove_btn = ttk.Button(self, text="✕", width=3, command=self._do_remove)
        self.remove_btn.pack(side=tk.LEFT)

        self._validate_formula_display()

    def _on_formula_write(self, *_args: Any) -> None:
        self._validate_formula_display()
        self.on_changed()

    def _on_percentage_write(self, *_args: Any) -> None:
        self.on_changed()

    def _validate_formula_display(self) -> None:
        f = self.formula_var.get().strip()
        if not f:
            self.formula_status.configure(text="", foreground="#666666")
            return
        valid = elements.is_valid_formula(f)
        if valid:
            self.formula_status.configure(text="Válida", foreground="#2E7D32")
        else:
            self.formula_status.configure(text="Inválida", foreground="#C62828")

    def _do_remove(self) -> None:
        self.on_remove(self)

    def get_data(self) -> Tuple[str, Optional[float]]:
        f = self.formula_var.get().strip()
        p_raw = self.percentage_var.get().strip()
        if not p_raw:
            return f, None
        try:
            return f, float(p_raw)
        except ValueError:
            return f, None


class FrameAtmosphere(ttk.Frame):
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
        self.current_atmosphere_id: str = sql_builder.generate_uuid()
        self.planets_map: Dict[str, str] = {}
        self.component_rows: List[GasComponentRow] = []

        self.columnconfigure(0, weight=1)

        planet_frame = ttk.LabelFrame(self, text="Planeta Associado", padding=10)
        planet_frame.grid(row=0, column=0, sticky="ew", padx=6, pady=4)

        ttk.Label(planet_frame, text="Planeta:", width=22, anchor="w").pack(side=tk.LEFT)
        self.planet_var = tk.StringVar()
        self.planet_combo = ttk.Combobox(
            planet_frame,
            textvariable=self.planet_var,
            values=[],
            state="readonly",
            width=38,
        )
        self.planet_combo.pack(side=tk.LEFT, padx=(0, 6))

        ttk.Button(planet_frame, text="Atualizar", command=self.refresh_planets).pack(side=tk.LEFT)

        params_frame = ttk.LabelFrame(self, text="Parâmetros Termodinâmicos", padding=10)
        params_frame.grid(row=1, column=0, sticky="ew", padx=6, pady=4)
        params_frame.columnconfigure(0, weight=1)

        id_row = ttk.Frame(params_frame)
        id_row.grid(row=0, column=0, sticky="ew", pady=3)
        ttk.Label(id_row, text="UUID da Atmosfera:", width=22, anchor="w").pack(side=tk.LEFT)
        self.id_var = tk.StringVar(value=self.current_atmosphere_id)
        self.id_entry = ttk.Entry(id_row, textvariable=self.id_var, width=38)
        self.id_entry.pack(side=tk.LEFT, padx=(0, 6))
        ttk.Button(id_row, text="Novo UUID", command=self._regenerate_uuid).pack(side=tk.LEFT)

        self.entry_pressure = UnitEntry(
            params_frame,
            "Pressão Superficial:",
            [("Pa", lambda x: x), ("atm", units.atm_to_pa), ("bar", units.bar_to_pa)],
            default_unit_idx=0,
        )
        self.entry_pressure.grid(row=1, column=0, sticky="w", pady=3)

        self.entry_greenhouse = UnitEntry(
            params_frame,
            "Efeito Estufa (ΔT):",
            [("K", lambda x: x)],
            default_unit_idx=0,
        )
        self.entry_greenhouse.grid(row=2, column=0, sticky="w", pady=3)

        self.entry_lapse_rate = UnitEntry(
            params_frame,
            "Lapse Rate (Γ):",
            [("K/m", lambda x: x), ("K/km", lambda x: x / 1000.0)],
            default_unit_idx=0,
        )
        self.entry_lapse_rate.grid(row=3, column=0, sticky="w", pady=3)

        comp_frame = ttk.LabelFrame(self, text="Composição Gasosa", padding=10)
        comp_frame.grid(row=2, column=0, sticky="nsew", padx=6, pady=4)
        comp_frame.columnconfigure(0, weight=1)

        comp_header = ttk.Frame(comp_frame)
        comp_header.pack(fill=tk.X, pady=(0, 4))
        ttk.Label(comp_header, text="Fórmula Molecular", width=16, font=("Segoe UI", 9, "bold")).pack(side=tk.LEFT)
        ttk.Label(comp_header, text="Status", width=8, font=("Segoe UI", 9, "bold")).pack(side=tk.LEFT)
        ttk.Label(comp_header, text="Percentual (%)", width=16, font=("Segoe UI", 9, "bold")).pack(side=tk.LEFT)

        self.components_container = ttk.Frame(comp_frame)
        self.components_container.pack(fill=tk.BOTH, expand=True)

        comp_footer = ttk.Frame(comp_frame)
        comp_footer.pack(fill=tk.X, pady=(8, 0))

        ttk.Button(comp_footer, text="+ Adicionar Gás", command=self.add_gas_row).pack(side=tk.LEFT, padx=(0, 12))

        self.total_percentage_lbl = ttk.Label(
            comp_footer,
            text="Total: 0.00 %",
            font=("Segoe UI", 9, "bold"),
        )
        self.total_percentage_lbl.pack(side=tk.LEFT)

        actions_frame = ttk.Frame(self)
        actions_frame.grid(row=3, column=0, sticky="ew", padx=6, pady=10)

        ttk.Button(
            actions_frame,
            text="Gerar SQL",
            command=self.generate_sql,
        ).pack(side=tk.LEFT, padx=(0, 6))

        ttk.Button(
            actions_frame,
            text="Registrar no Cache",
            command=self.register_in_cache,
        ).pack(side=tk.LEFT, padx=(0, 6))

        ttk.Button(
            actions_frame,
            text="Limpar Formulário",
            command=self.clear_form,
        ).pack(side=tk.LEFT)

        self.refresh_planets()
        self._add_default_gases()

    def _regenerate_uuid(self) -> None:
        self.current_atmosphere_id = sql_builder.generate_uuid()
        self.id_var.set(self.current_atmosphere_id)

    def refresh_planets(self) -> None:
        existing_atms = {a["planet_id"] for a in cache.list_atmospheres()}
        all_planets = cache.list_planets()

        self.planets_map.clear()
        displays = []
        for p in all_planets:
            p_id = p["id"]
            d = f"{p['name']} ({p_id[:8]}...)"
            if p_id in existing_atms:
                d += " [Já tem atmosfera]"
            self.planets_map[d] = p_id
            displays.append(d)

        self.planet_combo.configure(values=displays)
        if displays:
            if self.planet_var.get() not in displays:
                self.planet_var.set(displays[0])
        else:
            self.planet_var.set("")

    def _add_default_gases(self) -> None:
        self.add_gas_row("N2", "78.08")
        self.add_gas_row("O2", "20.95")
        self.add_gas_row("Ar", "0.93")
        self.add_gas_row("CO2", "0.04")

    def add_gas_row(self, formula: str = "", percentage: str = "") -> None:
        row = GasComponentRow(
            self.components_container,
            on_remove=self.remove_gas_row,
            on_changed=self._update_total_percentage,
            initial_formula=formula,
            initial_percentage=percentage,
        )
        row.pack(fill=tk.X, pady=2)
        self.component_rows.append(row)
        self._update_total_percentage()

    def remove_gas_row(self, row: GasComponentRow) -> None:
        if row in self.component_rows:
            self.component_rows.remove(row)
            row.destroy()
            self._update_total_percentage()

    def _update_total_percentage(self) -> None:
        total = 0.0
        for row in self.component_rows:
            _, p = row.get_data()
            if p is not None:
                total += p

        if total > 100.001:
            self.total_percentage_lbl.configure(
                text=f"Total: {total:.2f} % (Excede 100%!)",
                foreground="#C62828",
            )
        elif abs(total - 100.0) < 0.001:
            self.total_percentage_lbl.configure(
                text=f"Total: {total:.2f} % (Completo)",
                foreground="#2E7D32",
            )
        else:
            self.total_percentage_lbl.configure(
                text=f"Total: {total:.2f} %",
                foreground="#000000",
            )

    def build_model(self) -> Tuple[Atmosphere, List[AtmosphereGasComponent]]:
        planet_id = self.planets_map.get(self.planet_var.get(), "")
        atm_id = self.id_var.get().strip()

        atmosphere = Atmosphere(
            id=atm_id,
            planet_id=planet_id,
            pressure_pa=self.entry_pressure.get_si_value() or 0.0,
            greenhouse_effect_k=self.entry_greenhouse.get_si_value() or 0.0,
            lapse_rate_k_per_m=self.entry_lapse_rate.get_si_value() or 0.0,
        )

        components: List[AtmosphereGasComponent] = []
        for row in self.component_rows:
            formula, pct = row.get_data()
            if formula or pct is not None:
                components.append(
                    AtmosphereGasComponent(
                        atmosphere_id=atm_id,
                        formula=formula,
                        percentage=pct if pct is not None else float("nan"),
                    )
                )

        return atmosphere, components

    def generate_sql(self) -> Optional[Tuple[Atmosphere, List[AtmosphereGasComponent]]]:
        model, components = self.build_model()
        errors = validate_atmosphere(model, components)

        if errors:
            messagebox.showerror("Erro de Validação", "\n".join(f"• {e}" for e in errors))
            return None

        sql = sql_builder.build_insert_atmosphere(model, components=components, atomic=True)
        self.output_panel.append_sql(sql)
        return model, components

    def register_in_cache(self) -> None:
        res = self.generate_sql()
        if not res:
            return

        model, _ = res
        cache.register_atmosphere(model.id, model.planet_id)
        if self.on_cache_updated:
            self.on_cache_updated()
        self.refresh_planets()
        messagebox.showinfo("Sucesso", f"Atmosfera para o planeta registrada no cache!")

    def clear_form(self) -> None:
        self._regenerate_uuid()
        self.entry_pressure.clear()
        self.entry_greenhouse.clear()
        self.entry_lapse_rate.clear()
        for r in list(self.component_rows):
            r.destroy()
        self.component_rows.clear()
        self._add_default_gases()
