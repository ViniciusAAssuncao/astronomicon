import math
import tkinter as tk
from tkinter import ttk
from typing import Any, Callable, Dict, List, Optional, Tuple

import cache
import units
from models import OrbitalElements


class UnitEntry(ttk.Frame):
    def __init__(
        self,
        parent: tk.Widget,
        label_text: str,
        unit_options: List[Tuple[str, Callable[[float], float]]],
        default_unit_idx: int = 0,
        initial_value: str = "",
        width: int = 15,
        **kwargs: Any,
    ) -> None:
        super().__init__(parent, **kwargs)
        self.unit_options = unit_options
        self.unit_names = [opt[0] for opt in unit_options]
        self.converters = {opt[0]: opt[1] for opt in unit_options}

        self.label = ttk.Label(self, text=label_text, width=22, anchor="w")
        self.label.pack(side=tk.LEFT, padx=(0, 4))

        self.value_var = tk.StringVar(value=initial_value)
        self.entry = ttk.Entry(self, textvariable=self.value_var, width=width)
        self.entry.pack(side=tk.LEFT, padx=(0, 4))

        self.unit_var = tk.StringVar(value=self.unit_names[default_unit_idx])
        self.unit_combo = ttk.Combobox(
            self,
            textvariable=self.unit_var,
            values=self.unit_names,
            state="readonly",
            width=8,
        )
        self.unit_combo.pack(side=tk.LEFT)

    def get_si_value(self) -> Optional[float]:
        raw_text = self.value_var.get().strip()
        if not raw_text:
            return None
        try:
            val = float(raw_text)
            converter = self.converters[self.unit_var.get()]
            return converter(val)
        except (ValueError, KeyError):
            return None

    def get_raw_value(self) -> Optional[float]:
        raw_text = self.value_var.get().strip()
        if not raw_text:
            return None
        try:
            return float(raw_text)
        except ValueError:
            return None

    def set_value(self, val: str, unit: Optional[str] = None) -> None:
        self.value_var.set(val)
        if unit and unit in self.unit_names:
            self.unit_var.set(unit)

    def clear(self) -> None:
        self.value_var.set("")


class OrbitalElementsFrame(ttk.LabelFrame):
    def __init__(
        self,
        parent: tk.Widget,
        title: str = "Elementos Orbitais",
        **kwargs: Any,
    ) -> None:
        super().__init__(parent, text=title, padding=8, **kwargs)

        self.columnconfigure(0, weight=1)
        self.columnconfigure(1, weight=1)

        self.entry_semi_major = UnitEntry(
            self,
            "Semi-eixo maior (a):",
            [("AU", units.au_to_meters), ("m", lambda x: x), ("km", units.km_to_meters)],
            default_unit_idx=0,
        )
        self.entry_semi_major.grid(row=0, column=0, sticky="ew", padx=4, pady=3)

        self.entry_eccentricity = UnitEntry(
            self,
            "Excentricidade (e):",
            [("-", lambda x: x)],
            default_unit_idx=0,
        )
        self.entry_eccentricity.grid(row=0, column=1, sticky="ew", padx=4, pady=3)

        self.entry_inclination = UnitEntry(
            self,
            "Inclinação (i):",
            [("graus", units.degrees_to_radians), ("rad", lambda x: x)],
            default_unit_idx=0,
        )
        self.entry_inclination.grid(row=1, column=0, sticky="ew", padx=4, pady=3)

        self.entry_lan = UnitEntry(
            self,
            "Nodo Ascendente (Ω):",
            [("graus", units.degrees_to_radians), ("rad", lambda x: x)],
            default_unit_idx=0,
        )
        self.entry_lan.grid(row=1, column=1, sticky="ew", padx=4, pady=3)

        self.entry_arg_periapsis = UnitEntry(
            self,
            "Arg. Periastro (ω):",
            [("graus", units.degrees_to_radians), ("rad", lambda x: x)],
            default_unit_idx=0,
        )
        self.entry_arg_periapsis.grid(row=2, column=0, sticky="ew", padx=4, pady=3)

        self.entry_mean_anomaly = UnitEntry(
            self,
            "Anomalia Média (M0):",
            [("graus", units.degrees_to_radians), ("rad", lambda x: x)],
            default_unit_idx=0,
        )
        self.entry_mean_anomaly.grid(row=2, column=1, sticky="ew", padx=4, pady=3)

    def get_elements(self) -> Optional[OrbitalElements]:
        a = self.entry_semi_major.get_si_value()
        e = self.entry_eccentricity.get_si_value()
        inc = self.entry_inclination.get_si_value()
        lan = self.entry_lan.get_si_value()
        arg = self.entry_arg_periapsis.get_si_value()
        m0 = self.entry_mean_anomaly.get_si_value()

        if None in (a, e, inc, lan, arg, m0):
            return None

        return OrbitalElements(
            semi_major_axis_m=a,  # type: ignore
            eccentricity=e,  # type: ignore
            inclination_rad=inc,  # type: ignore
            longitude_ascending_node_rad=lan,  # type: ignore
            argument_periapsis_rad=arg,  # type: ignore
            mean_anomaly_at_epoch_rad=m0,  # type: ignore
        )

    def is_empty(self) -> bool:
        values = [
            self.entry_semi_major.value_var.get().strip(),
            self.entry_eccentricity.value_var.get().strip(),
            self.entry_inclination.value_var.get().strip(),
            self.entry_lan.value_var.get().strip(),
            self.entry_arg_periapsis.value_var.get().strip(),
            self.entry_mean_anomaly.value_var.get().strip(),
        ]
        return all(v == "" for v in values)

    def set_enabled(self, enabled: bool) -> None:
        state = "normal" if enabled else "disabled"
        for child in (
            self.entry_semi_major,
            self.entry_eccentricity,
            self.entry_inclination,
            self.entry_lan,
            self.entry_arg_periapsis,
            self.entry_mean_anomaly,
        ):
            child.entry.configure(state=state)
            child.unit_combo.configure(state="readonly" if enabled else "disabled")

    def clear(self) -> None:
        self.entry_semi_major.clear()
        self.entry_eccentricity.clear()
        self.entry_inclination.clear()
        self.entry_lan.clear()
        self.entry_arg_periapsis.clear()
        self.entry_mean_anomaly.clear()


class OrbitalParentSelector(ttk.Frame):
    def __init__(
        self,
        parent: tk.Widget,
        on_parent_type_changed: Optional[Callable[[str], None]] = None,
        allowed_parent_types: Optional[List[str]] = None,
        **kwargs: Any,
    ) -> None:
        super().__init__(parent, **kwargs)

        self.on_parent_type_changed = on_parent_type_changed
        self.system_id: Optional[str] = None
        self.entities_map: Dict[str, str] = {}

        types = ["Fixo (sem pai)"]
        if allowed_parent_types is None:
            types.extend(["Estrela", "Planeta", "Baricentro"])
        else:
            types.extend(allowed_parent_types)

        lbl = ttk.Label(self, text="Parentesco Orbital:", width=20, anchor="w")
        lbl.pack(side=tk.LEFT, padx=(0, 4))

        self.parent_type_var = tk.StringVar(value="Fixo (sem pai)")
        self.type_combo = ttk.Combobox(
            self,
            textvariable=self.parent_type_var,
            values=types,
            state="readonly",
            width=16,
        )
        self.type_combo.pack(side=tk.LEFT, padx=(0, 6))
        self.type_combo.bind("<<ComboboxSelected>>", self._on_type_selected)

        self.entity_var = tk.StringVar()
        self.entity_combo = ttk.Combobox(
            self,
            textvariable=self.entity_var,
            values=[],
            state="disabled",
            width=28,
        )
        self.entity_combo.pack(side=tk.LEFT)

    def set_system_id(self, system_id: Optional[str]) -> None:
        self.system_id = system_id
        self._refresh_entity_list()

    def _on_type_selected(self, _event: Optional[tk.Event] = None) -> None:
        selected_type = self.parent_type_var.get()
        if selected_type == "Fixo (sem pai)":
            self.entity_combo.configure(state="disabled")
            self.entity_var.set("")
        else:
            self.entity_combo.configure(state="readonly")
            self._refresh_entity_list()

        if self.on_parent_type_changed:
            self.on_parent_type_changed(selected_type)

    def _refresh_entity_list(self) -> None:
        selected_type = self.parent_type_var.get()
        self.entities_map.clear()

        if selected_type == "Estrela":
            rows = cache.list_stars(star_system_id=self.system_id)
            for r in rows:
                display = f"{r['name']} ({r['id'][:8]}...)"
                self.entities_map[display] = r["id"]
        elif selected_type == "Planeta":
            rows = cache.list_planets(star_system_id=self.system_id)
            for r in rows:
                display = f"{r['name']} ({r['id'][:8]}...)"
                self.entities_map[display] = r["id"]
        elif selected_type == "Baricentro":
            rows = cache.list_barycenters(star_system_id=self.system_id)
            for r in rows:
                display = f"{r['name']} ({r['id'][:8]}...)"
                self.entities_map[display] = r["id"]

        displays = list(self.entities_map.keys())
        self.entity_combo.configure(values=displays)
        if displays:
            if self.entity_var.get() not in displays:
                self.entity_var.set(displays[0])
        else:
            self.entity_var.set("")

    def get_parent_references(self) -> Tuple[Optional[str], Optional[str], Optional[str]]:
        selected_type = self.parent_type_var.get()
        if selected_type == "Fixo (sem pai)":
            return None, None, None

        display = self.entity_var.get()
        entity_id = self.entities_map.get(display)
        if not entity_id:
            return None, None, None

        if selected_type == "Estrela":
            return entity_id, None, None
        if selected_type == "Planeta":
            return None, entity_id, None
        if selected_type == "Baricentro":
            return None, None, entity_id
        return None, None, None

    def is_fixed(self) -> bool:
        return self.parent_type_var.get() == "Fixo (sem pai)"

    def clear(self) -> None:
        self.parent_type_var.set("Fixo (sem pai)")
        self.entity_combo.configure(state="disabled")
        self.entity_var.set("")
        if self.on_parent_type_changed:
            self.on_parent_type_changed("Fixo (sem pai)")


class BarycenterMemberSelector(ttk.Frame):
    def __init__(
        self,
        parent: tk.Widget,
        label_text: str,
        on_member_changed: Optional[Callable[[], None]] = None,
        **kwargs: Any,
    ) -> None:
        super().__init__(parent, **kwargs)

        self.on_member_changed = on_member_changed
        self.system_id: Optional[str] = None
        self.entities_map: Dict[str, str] = {}

        lbl = ttk.Label(self, text=label_text, width=20, anchor="w")
        lbl.pack(side=tk.LEFT, padx=(0, 4))

        self.member_type_var = tk.StringVar(value="Estrela")
        self.type_combo = ttk.Combobox(
            self,
            textvariable=self.member_type_var,
            values=["Estrela", "Planeta", "Baricentro"],
            state="readonly",
            width=14,
        )
        self.type_combo.pack(side=tk.LEFT, padx=(0, 6))
        self.type_combo.bind("<<ComboboxSelected>>", self._on_type_selected)

        self.entity_var = tk.StringVar()
        self.entity_combo = ttk.Combobox(
            self,
            textvariable=self.entity_var,
            values=[],
            state="readonly",
            width=28,
        )
        self.entity_combo.pack(side=tk.LEFT)
        self.entity_combo.bind("<<ComboboxSelected>>", self._on_entity_selected)

    def set_system_id(self, system_id: Optional[str]) -> None:
        self.system_id = system_id
        self._refresh_entity_list()

    def _on_type_selected(self, _event: Optional[tk.Event] = None) -> None:
        self._refresh_entity_list()
        if self.on_member_changed:
            self.on_member_changed()

    def _on_entity_selected(self, _event: Optional[tk.Event] = None) -> None:
        if self.on_member_changed:
            self.on_member_changed()

    def _refresh_entity_list(self) -> None:
        selected_type = self.member_type_var.get()
        self.entities_map.clear()

        if selected_type == "Estrela":
            rows = cache.list_stars(star_system_id=self.system_id)
            for r in rows:
                display = f"{r['name']} ({r['id'][:8]}...)"
                self.entities_map[display] = r["id"]
        elif selected_type == "Planeta":
            rows = cache.list_planets(star_system_id=self.system_id)
            for r in rows:
                display = f"{r['name']} ({r['id'][:8]}...)"
                self.entities_map[display] = r["id"]
        elif selected_type == "Baricentro":
            rows = cache.list_barycenters(star_system_id=self.system_id)
            for r in rows:
                display = f"{r['name']} ({r['id'][:8]}...)"
                self.entities_map[display] = r["id"]

        displays = list(self.entities_map.keys())
        self.entity_combo.configure(values=displays)
        if displays:
            if self.entity_var.get() not in displays:
                self.entity_var.set(displays[0])
        else:
            self.entity_var.set("")

    def get_member_references(self) -> Tuple[Optional[str], Optional[str], Optional[str]]:
        display = self.entity_var.get()
        entity_id = self.entities_map.get(display)
        if not entity_id:
            return None, None, None

        selected_type = self.member_type_var.get()
        if selected_type == "Estrela":
            return entity_id, None, None
        if selected_type == "Planeta":
            return None, entity_id, None
        if selected_type == "Baricentro":
            return None, None, entity_id
        return None, None, None

    def get_entity_id(self) -> Optional[str]:
        display = self.entity_var.get()
        return self.entities_map.get(display)

    def clear(self) -> None:
        self.member_type_var.set("Estrela")
        self.entity_var.set("")
        self._refresh_entity_list()
