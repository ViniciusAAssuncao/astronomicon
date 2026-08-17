import os
import sqlite3
from typing import Any, Dict, List, Optional, Tuple

from models import Atmosphere, Barycenter, Planet, Star, StarSystem

DEFAULT_CACHE_DB = os.path.join(os.path.dirname(os.path.abspath(__file__)), "editor_cache.db")


def get_connection(db_path: Optional[str] = None) -> sqlite3.Connection:
    path = db_path if db_path is not None else DEFAULT_CACHE_DB
    conn = sqlite3.connect(path)
    conn.row_factory = sqlite3.Row
    init_cache(conn)
    return conn


def init_cache(conn: sqlite3.Connection) -> None:
    cur = conn.cursor()
    cur.execute(
        """
        CREATE TABLE IF NOT EXISTS star_systems (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL
        )
        """
    )
    cur.execute(
        """
        CREATE TABLE IF NOT EXISTS stars (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            kind TEXT NOT NULL,
            star_system_id TEXT,
            parent_star_id TEXT,
            parent_planet_id TEXT,
            parent_barycenter_id TEXT
        )
        """
    )
    cur.execute(
        """
        CREATE TABLE IF NOT EXISTS planets (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            kind TEXT NOT NULL,
            star_system_id TEXT,
            parent_star_id TEXT,
            parent_planet_id TEXT,
            parent_barycenter_id TEXT
        )
        """
    )
    cur.execute(
        """
        CREATE TABLE IF NOT EXISTS barycenters (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            star_system_id TEXT,
            primary_id TEXT,
            primary_type TEXT,
            secondary_id TEXT,
            secondary_type TEXT,
            parent_star_id TEXT,
            parent_planet_id TEXT,
            parent_barycenter_id TEXT
        )
        """
    )
    cur.execute(
        """
        CREATE TABLE IF NOT EXISTS atmospheres (
            id TEXT PRIMARY KEY,
            planet_id TEXT UNIQUE NOT NULL
        )
        """
    )
    conn.commit()


def register_star_system(
    system_id: str,
    name: str,
    db_path: Optional[str] = None,
) -> None:
    conn = get_connection(db_path)
    cur = conn.cursor()
    cur.execute(
        """
        INSERT INTO star_systems (id, name)
        VALUES (?, ?)
        ON CONFLICT(id) DO UPDATE SET name = excluded.name
        """,
        (system_id, name),
    )
    conn.commit()
    conn.close()


def register_star(
    star_id: str,
    name: str,
    kind: str,
    star_system_id: Optional[str] = None,
    parent_star_id: Optional[str] = None,
    parent_planet_id: Optional[str] = None,
    parent_barycenter_id: Optional[str] = None,
    db_path: Optional[str] = None,
) -> None:
    conn = get_connection(db_path)
    cur = conn.cursor()
    cur.execute(
        """
        INSERT INTO stars (
            id, name, kind, star_system_id,
            parent_star_id, parent_planet_id, parent_barycenter_id
        )
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            kind = excluded.kind,
            star_system_id = excluded.star_system_id,
            parent_star_id = excluded.parent_star_id,
            parent_planet_id = excluded.parent_planet_id,
            parent_barycenter_id = excluded.parent_barycenter_id
        """,
        (
            star_id,
            name,
            kind,
            star_system_id,
            parent_star_id,
            parent_planet_id,
            parent_barycenter_id,
        ),
    )
    conn.commit()
    conn.close()


def register_planet(
    planet_id: str,
    name: str,
    kind: str,
    star_system_id: Optional[str] = None,
    parent_star_id: Optional[str] = None,
    parent_planet_id: Optional[str] = None,
    parent_barycenter_id: Optional[str] = None,
    db_path: Optional[str] = None,
) -> None:
    conn = get_connection(db_path)
    cur = conn.cursor()
    cur.execute(
        """
        INSERT INTO planets (
            id, name, kind, star_system_id,
            parent_star_id, parent_planet_id, parent_barycenter_id
        )
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            kind = excluded.kind,
            star_system_id = excluded.star_system_id,
            parent_star_id = excluded.parent_star_id,
            parent_planet_id = excluded.parent_planet_id,
            parent_barycenter_id = excluded.parent_barycenter_id
        """,
        (
            planet_id,
            name,
            kind,
            star_system_id,
            parent_star_id,
            parent_planet_id,
            parent_barycenter_id,
        ),
    )
    conn.commit()
    conn.close()


def register_barycenter(
    barycenter_id: str,
    name: str,
    star_system_id: Optional[str] = None,
    primary_id: Optional[str] = None,
    primary_type: Optional[str] = None,
    secondary_id: Optional[str] = None,
    secondary_type: Optional[str] = None,
    parent_star_id: Optional[str] = None,
    parent_planet_id: Optional[str] = None,
    parent_barycenter_id: Optional[str] = None,
    db_path: Optional[str] = None,
) -> None:
    conn = get_connection(db_path)
    cur = conn.cursor()
    cur.execute(
        """
        INSERT INTO barycenters (
            id, name, star_system_id,
            primary_id, primary_type,
            secondary_id, secondary_type,
            parent_star_id, parent_planet_id, parent_barycenter_id
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            star_system_id = excluded.star_system_id,
            primary_id = excluded.primary_id,
            primary_type = excluded.primary_type,
            secondary_id = excluded.secondary_id,
            secondary_type = excluded.secondary_type,
            parent_star_id = excluded.parent_star_id,
            parent_planet_id = excluded.parent_planet_id,
            parent_barycenter_id = excluded.parent_barycenter_id
        """,
        (
            barycenter_id,
            name,
            star_system_id,
            primary_id,
            primary_type,
            secondary_id,
            secondary_type,
            parent_star_id,
            parent_planet_id,
            parent_barycenter_id,
        ),
    )
    conn.commit()
    conn.close()


def register_atmosphere(
    atmosphere_id: str,
    planet_id: str,
    db_path: Optional[str] = None,
) -> None:
    conn = get_connection(db_path)
    cur = conn.cursor()
    cur.execute(
        """
        INSERT INTO atmospheres (id, planet_id)
        VALUES (?, ?)
        ON CONFLICT(id) DO UPDATE SET planet_id = excluded.planet_id
        """,
        (atmosphere_id, planet_id),
    )
    conn.commit()
    conn.close()


def register_entity_from_model(model: Any, db_path: Optional[str] = None) -> None:
    if isinstance(model, StarSystem):
        register_star_system(model.id, model.name, db_path=db_path)
    elif isinstance(model, Star):
        register_star(
            star_id=model.id,
            name=model.name,
            kind=model.kind,
            star_system_id=model.star_system_id,
            parent_star_id=model.parent_star_id,
            parent_planet_id=model.parent_planet_id,
            parent_barycenter_id=model.parent_barycenter_id,
            db_path=db_path,
        )
    elif isinstance(model, Planet):
        register_planet(
            planet_id=model.id,
            name=model.name,
            kind=model.kind,
            star_system_id=model.star_system_id,
            parent_star_id=model.parent_star_id,
            parent_planet_id=model.parent_planet_id,
            parent_barycenter_id=model.parent_barycenter_id,
            db_path=db_path,
        )
    elif isinstance(model, Barycenter):
        primary_id = model.primary_star_id or model.primary_planet_id or model.primary_barycenter_id
        primary_type = (
            "Star"
            if model.primary_star_id
            else "Planet"
            if model.primary_planet_id
            else "Barycenter"
            if model.primary_barycenter_id
            else None
        )
        secondary_id = (
            model.secondary_star_id or model.secondary_planet_id or model.secondary_barycenter_id
        )
        secondary_type = (
            "Star"
            if model.secondary_star_id
            else "Planet"
            if model.secondary_planet_id
            else "Barycenter"
            if model.secondary_barycenter_id
            else None
        )
        register_barycenter(
            barycenter_id=model.id,
            name=model.name,
            star_system_id=model.star_system_id,
            primary_id=primary_id,
            primary_type=primary_type,
            secondary_id=secondary_id,
            secondary_type=secondary_type,
            parent_star_id=model.parent_star_id,
            parent_planet_id=model.parent_planet_id,
            parent_barycenter_id=model.parent_barycenter_id,
            db_path=db_path,
        )
    elif isinstance(model, Atmosphere):
        register_atmosphere(model.id, model.planet_id, db_path=db_path)


def register_manual_entity(
    entity_type: str,
    entity_id: str,
    name: str,
    kind: Optional[str] = None,
    star_system_id: Optional[str] = None,
    parent_star_id: Optional[str] = None,
    parent_planet_id: Optional[str] = None,
    parent_barycenter_id: Optional[str] = None,
    primary_id: Optional[str] = None,
    primary_type: Optional[str] = None,
    secondary_id: Optional[str] = None,
    secondary_type: Optional[str] = None,
    db_path: Optional[str] = None,
) -> None:
    norm_type = entity_type.strip().lower()
    if norm_type in ("starsystem", "star_system", "system"):
        register_star_system(entity_id, name, db_path=db_path)
    elif norm_type == "star":
        register_star(
            entity_id,
            name,
            kind or "Star",
            star_system_id,
            parent_star_id,
            parent_planet_id,
            parent_barycenter_id,
            db_path=db_path,
        )
    elif norm_type == "planet":
        register_planet(
            entity_id,
            name,
            kind or "Telluric",
            star_system_id,
            parent_star_id,
            parent_planet_id,
            parent_barycenter_id,
            db_path=db_path,
        )
    elif norm_type == "barycenter":
        register_barycenter(
            entity_id,
            name,
            star_system_id,
            primary_id,
            primary_type,
            secondary_id,
            secondary_type,
            parent_star_id,
            parent_planet_id,
            parent_barycenter_id,
            db_path=db_path,
        )
    elif norm_type == "atmosphere":
        register_atmosphere(entity_id, parent_planet_id or primary_id or "", db_path=db_path)
    else:
        raise ValueError(f"unknown entity type: {entity_type}")


def list_star_systems(db_path: Optional[str] = None) -> List[Dict[str, Any]]:
    conn = get_connection(db_path)
    cur = conn.cursor()
    cur.execute("SELECT id, name FROM star_systems ORDER BY name ASC")
    rows = [dict(row) for row in cur.fetchall()]
    conn.close()
    return rows


def list_stars(
    star_system_id: Optional[str] = None,
    db_path: Optional[str] = None,
) -> List[Dict[str, Any]]:
    conn = get_connection(db_path)
    cur = conn.cursor()
    if star_system_id:
        cur.execute(
            """
            SELECT id, name, kind, star_system_id,
                   parent_star_id, parent_planet_id, parent_barycenter_id
            FROM stars WHERE star_system_id = ? ORDER BY name ASC
            """,
            (star_system_id,),
        )
    else:
        cur.execute(
            """
            SELECT id, name, kind, star_system_id,
                   parent_star_id, parent_planet_id, parent_barycenter_id
            FROM stars ORDER BY name ASC
            """
        )
    rows = [dict(row) for row in cur.fetchall()]
    conn.close()
    return rows


def list_planets(
    star_system_id: Optional[str] = None,
    db_path: Optional[str] = None,
) -> List[Dict[str, Any]]:
    conn = get_connection(db_path)
    cur = conn.cursor()
    if star_system_id:
        cur.execute(
            """
            SELECT id, name, kind, star_system_id,
                   parent_star_id, parent_planet_id, parent_barycenter_id
            FROM planets WHERE star_system_id = ? ORDER BY name ASC
            """,
            (star_system_id,),
        )
    else:
        cur.execute(
            """
            SELECT id, name, kind, star_system_id,
                   parent_star_id, parent_planet_id, parent_barycenter_id
            FROM planets ORDER BY name ASC
            """
        )
    rows = [dict(row) for row in cur.fetchall()]
    conn.close()
    return rows


def list_barycenters(
    star_system_id: Optional[str] = None,
    db_path: Optional[str] = None,
) -> List[Dict[str, Any]]:
    conn = get_connection(db_path)
    cur = conn.cursor()
    if star_system_id:
        cur.execute(
            """
            SELECT id, name, star_system_id,
                   primary_id, primary_type,
                   secondary_id, secondary_type,
                   parent_star_id, parent_planet_id, parent_barycenter_id
            FROM barycenters WHERE star_system_id = ? ORDER BY name ASC
            """,
            (star_system_id,),
        )
    else:
        cur.execute(
            """
            SELECT id, name, star_system_id,
                   primary_id, primary_type,
                   secondary_id, secondary_type,
                   parent_star_id, parent_planet_id, parent_barycenter_id
            FROM barycenters ORDER BY name ASC
            """
        )
    rows = [dict(row) for row in cur.fetchall()]
    conn.close()
    return rows


def list_atmospheres(db_path: Optional[str] = None) -> List[Dict[str, Any]]:
    conn = get_connection(db_path)
    cur = conn.cursor()
    cur.execute("SELECT id, planet_id FROM atmospheres ORDER BY id ASC")
    rows = [dict(row) for row in cur.fetchall()]
    conn.close()
    return rows


def list_entities_by_type(
    entity_type: str,
    star_system_id: Optional[str] = None,
    db_path: Optional[str] = None,
) -> List[Dict[str, Any]]:
    norm_type = entity_type.strip().lower()
    if norm_type in ("starsystem", "star_system", "system"):
        return list_star_systems(db_path=db_path)
    if norm_type == "star":
        return list_stars(star_system_id=star_system_id, db_path=db_path)
    if norm_type == "planet":
        return list_planets(star_system_id=star_system_id, db_path=db_path)
    if norm_type == "barycenter":
        return list_barycenters(star_system_id=star_system_id, db_path=db_path)
    if norm_type == "atmosphere":
        return list_atmospheres(db_path=db_path)
    raise ValueError(f"unknown entity type: {entity_type}")


def get_entity(entity_id: str, db_path: Optional[str] = None) -> Optional[Dict[str, Any]]:
    conn = get_connection(db_path)
    cur = conn.cursor()

    cur.execute("SELECT id, name FROM star_systems WHERE id = ?", (entity_id,))
    row = cur.fetchone()
    if row:
        conn.close()
        res = dict(row)
        res["entity_type"] = "StarSystem"
        return res

    cur.execute(
        """
        SELECT id, name, kind, star_system_id,
               parent_star_id, parent_planet_id, parent_barycenter_id
        FROM stars WHERE id = ?
        """,
        (entity_id,),
    )
    row = cur.fetchone()
    if row:
        conn.close()
        res = dict(row)
        res["entity_type"] = "Star"
        return res

    cur.execute(
        """
        SELECT id, name, kind, star_system_id,
               parent_star_id, parent_planet_id, parent_barycenter_id
        FROM planets WHERE id = ?
        """,
        (entity_id,),
    )
    row = cur.fetchone()
    if row:
        conn.close()
        res = dict(row)
        res["entity_type"] = "Planet"
        return res

    cur.execute(
        """
        SELECT id, name, star_system_id,
               primary_id, primary_type,
               secondary_id, secondary_type,
               parent_star_id, parent_planet_id, parent_barycenter_id
        FROM barycenters WHERE id = ?
        """,
        (entity_id,),
    )
    row = cur.fetchone()
    if row:
        conn.close()
        res = dict(row)
        res["entity_type"] = "Barycenter"
        return res

    cur.execute("SELECT id, planet_id FROM atmospheres WHERE id = ?", (entity_id,))
    row = cur.fetchone()
    if row:
        conn.close()
        res = dict(row)
        res["entity_type"] = "Atmosphere"
        return res

    conn.close()
    return None


def is_barycenter_member(
    entity_id: str,
    db_path: Optional[str] = None,
) -> Tuple[bool, Optional[str], Optional[str]]:
    conn = get_connection(db_path)
    cur = conn.cursor()
    cur.execute(
        """
        SELECT id, name FROM barycenters
        WHERE primary_id = ? OR secondary_id = ?
        """,
        (entity_id, entity_id),
    )
    row = cur.fetchone()
    conn.close()
    if row:
        return True, row["id"], row["name"]
    return False, None, None


def check_direct_circular_parent(entity_id: str, prospective_parent_id: Optional[str]) -> bool:
    if not prospective_parent_id or not entity_id:
        return False
    return entity_id.strip() == prospective_parent_id.strip()


def check_circular_ancestry(
    entity_id: str,
    prospective_parent_id: Optional[str],
    db_path: Optional[str] = None,
) -> bool:
    if not prospective_parent_id or not entity_id:
        return False
    if check_direct_circular_parent(entity_id, prospective_parent_id):
        return True

    visited = set()
    current_id: Optional[str] = prospective_parent_id.strip()

    while current_id:
        if current_id == entity_id.strip():
            return True
        if current_id in visited:
            break
        visited.add(current_id)

        entity = get_entity(current_id, db_path=db_path)
        if not entity:
            break

        current_id = (
            entity.get("parent_star_id")
            or entity.get("parent_planet_id")
            or entity.get("parent_barycenter_id")
        )

    return False


def delete_entity(
    entity_type: str,
    entity_id: str,
    db_path: Optional[str] = None,
) -> bool:
    norm_type = entity_type.strip().lower()
    table_map = {
        "starsystem": "star_systems",
        "star_system": "star_systems",
        "system": "star_systems",
        "star": "stars",
        "planet": "planets",
        "barycenter": "barycenters",
        "atmosphere": "atmospheres",
    }
    table = table_map.get(norm_type)
    if not table:
        raise ValueError(f"unknown entity type: {entity_type}")

    conn = get_connection(db_path)
    cur = conn.cursor()
    cur.execute(f"DELETE FROM {table} WHERE id = ?", (entity_id,))
    deleted = cur.rowcount > 0
    conn.commit()
    conn.close()
    return deleted


def clear_cache(db_path: Optional[str] = None) -> None:
    conn = get_connection(db_path)
    cur = conn.cursor()
    cur.execute("DELETE FROM atmospheres")
    cur.execute("DELETE FROM barycenters")
    cur.execute("DELETE FROM planets")
    cur.execute("DELETE FROM stars")
    cur.execute("DELETE FROM star_systems")
    conn.commit()
    conn.close()


def run_manual_registration_console(db_path: Optional[str] = None) -> None:
    print("=== Astronomicon Editor - Registro Manual de Entidades ===")
    while True:
        print("\nSelecione o tipo de entidade:")
        print("1. Sistema Estelar (StarSystem)")
        print("2. Estrela (Star)")
        print("3. Planeta (Planet)")
        print("4. Baricentro (Barycenter)")
        print("5. Listar entidades no cache")
        print("6. Checar duplicidade em baricentros")
        print("7. Checar referências circulares de parentesco")
        print("0. Sair")

        choice = input("Opção: ").strip()
        if choice == "0":
            break

        if choice == "1":
            uid = input("UUID do Sistema: ").strip()
            name = input("Nome do Sistema: ").strip()
            if uid and name:
                register_star_system(uid, name, db_path=db_path)
                print(f"Sistema '{name}' ({uid}) registrado com sucesso.")
            else:
                print("Erro: UUID e Nome são obrigatórios.")

        elif choice == "2":
            uid = input("UUID da Estrela: ").strip()
            name = input("Nome da Estrela: ").strip()
            kind = input("Tipo (Star/WhiteDwarf/NeutronStar/BlackHole/BrownDwarf/Exotic) [Star]: ").strip() or "Star"
            sys_id = input("UUID do Sistema Estelar (opcional): ").strip() or None
            p_star = input("UUID Estrela Pai (opcional): ").strip() or None
            p_planet = input("UUID Planeta Pai (opcional): ").strip() or None
            p_bary = input("UUID Baricentro Pai (opcional): ").strip() or None
            if uid and name:
                register_star(
                    star_id=uid,
                    name=name,
                    kind=kind,
                    star_system_id=sys_id,
                    parent_star_id=p_star,
                    parent_planet_id=p_planet,
                    parent_barycenter_id=p_bary,
                    db_path=db_path,
                )
                print(f"Estrela '{name}' ({uid}) registrada com sucesso.")
            else:
                print("Erro: UUID e Nome são obrigatórios.")

        elif choice == "3":
            uid = input("UUID do Planeta: ").strip()
            name = input("Nome do Planeta: ").strip()
            kind = input("Tipo (Telluric/GasGiant/IceGiant/DwarfPlanet/Chthonian/CarbonPlanet/IcyBody/Exotic) [Telluric]: ").strip() or "Telluric"
            sys_id = input("UUID do Sistema Estelar (opcional): ").strip() or None
            p_star = input("UUID Estrela Pai (opcional): ").strip() or None
            p_planet = input("UUID Planeta Pai (opcional): ").strip() or None
            p_bary = input("UUID Baricentro Pai (opcional): ").strip() or None
            if uid and name:
                register_planet(
                    planet_id=uid,
                    name=name,
                    kind=kind,
                    star_system_id=sys_id,
                    parent_star_id=p_star,
                    parent_planet_id=p_planet,
                    parent_barycenter_id=p_bary,
                    db_path=db_path,
                )
                print(f"Planeta '{name}' ({uid}) registrado com sucesso.")
            else:
                print("Erro: UUID e Nome são obrigatórios.")

        elif choice == "4":
            uid = input("UUID do Baricentro: ").strip()
            name = input("Nome do Baricentro: ").strip()
            sys_id = input("UUID do Sistema Estelar (opcional): ").strip() or None
            pri_id = input("UUID Membro Primário: ").strip() or None
            pri_type = input("Tipo Primário (Star/Planet/Barycenter): ").strip() or None
            sec_id = input("UUID Membro Secundário: ").strip() or None
            sec_type = input("Tipo Secundário (Star/Planet/Barycenter): ").strip() or None
            p_star = input("UUID Estrela Pai (opcional): ").strip() or None
            p_planet = input("UUID Planeta Pai (opcional): ").strip() or None
            p_bary = input("UUID Baricentro Pai (opcional): ").strip() or None
            if uid and name:
                register_barycenter(
                    barycenter_id=uid,
                    name=name,
                    star_system_id=sys_id,
                    primary_id=pri_id,
                    primary_type=pri_type,
                    secondary_id=sec_id,
                    secondary_type=sec_type,
                    parent_star_id=p_star,
                    parent_planet_id=p_planet,
                    parent_barycenter_id=p_bary,
                    db_path=db_path,
                )
                print(f"Baricentro '{name}' ({uid}) registrado com sucesso.")
            else:
                print("Erro: UUID e Nome são obrigatórios.")

        elif choice == "5":
            print("\n--- Sistemas Estelares ---")
            for s in list_star_systems(db_path=db_path):
                print(f"  [{s['id']}] {s['name']}")
            print("\n--- Estrelas ---")
            for st in list_stars(db_path=db_path):
                print(f"  [{st['id']}] {st['name']} ({st['kind']}) - Sistema: {st['star_system_id']}")
            print("\n--- Planetas ---")
            for pl in list_planets(db_path=db_path):
                print(f"  [{pl['id']}] {pl['name']} ({pl['kind']}) - Sistema: {pl['star_system_id']}")
            print("\n--- Baricentros ---")
            for b in list_barycenters(db_path=db_path):
                print(f"  [{b['id']}] {b['name']} - Primário: {b['primary_id']} | Secundário: {b['secondary_id']}")

        elif choice == "6":
            uid = input("UUID da entidade a consultar: ").strip()
            is_mem, b_id, b_name = is_barycenter_member(uid, db_path=db_path)
            if is_mem:
                print(f"Entidade '{uid}' JÁ É MEMBRO do baricentro '{b_name}' ({b_id}).")
            else:
                print(f"Entidade '{uid}' NÃO é membro de nenhum baricentro no cache.")

        elif choice == "7":
            child_id = input("UUID da entidade filha: ").strip()
            parent_id = input("UUID da entidade pai proposta: ").strip()
            is_circ = check_circular_ancestry(child_id, parent_id, db_path=db_path)
            if is_circ:
                print(f"ALERTA: A relação criaria uma REFERÊNCIA CIRCULAR!")
            else:
                print("Hierarquia válida, sem referências circulares detectadas.")


if __name__ == "__main__":
    run_manual_registration_console()
