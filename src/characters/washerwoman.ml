open! Core
open! Import

include Character.Make (struct
    type t =
      { kind : Kind.t
      ; alignment : Alignment.t
      }

    let id = "washerwoman"
    let name = "Washerwoman"
    let t = { kind = Kind.Townsfolk; alignment = Alignment.Good }
    let kind { kind; _ } = kind
    let alignment { alignment; _ } = alignment
  end)

let night_action ~player_id:pid ~night =
  if night <> 1
  then None
  else
    Some
      (if_alive
         pid
         (let%bind.Botc_exec state = Botc_exec.get_state
          and () = Botc_exec.wake pid in
          let pname id = Player_id.to_string id in
          let%bind.Botc_exec () =
            if Game_state.is_poisoned state pid
            then (
              let ids = Game_state.alive_ids state in
              let%bind.Botc_exec players =
                narrator_pick "poisoned washerwoman players" ids ~pick_count:2
              in
              let p1 = List.nth_exn players 0 in
              let p2 = List.nth_exn players 1 in
              let tf_names =
                List.filter_map (Game_state.alive_ids state) ~f:(fun id ->
                  if Kind.equal (Game_state.kind state id) Kind.Townsfolk
                  then Some (Game_state.character_name state id)
                  else None)
              in
              let%bind.Botc_exec roles =
                Botc_exec.narrator_pick "poisoned washerwoman role" tf_names ~pick_count:1
              in
              let role = List.hd_exn roles in
              let n1 = pname p1 in
              let n2 = pname p2 in
              Botc_exec.tell pid [%string "%{n1} and %{n2} — one is the %{role}"])
            else (
              let townsfolk_ids =
                List.filter (Game_state.alive_ids state) ~f:(fun id ->
                  Kind.equal (Game_state.kind state id) Kind.Townsfolk)
              in
              let%bind.Botc_exec tf =
                narrator_pick "washerwoman townsfolk" townsfolk_ids ~pick_count:1
              in
              let tf_id = List.hd_exn tf in
              let other_ids = alive_except state tf_id in
              let%bind.Botc_exec others =
                narrator_pick "washerwoman other player" other_ids ~pick_count:1
              in
              let other = List.hd_exn others in
              let%bind.Botc_exec order =
                narrator_pick "washerwoman display order" [ false; true ] ~pick_count:1
              in
              let swap = List.hd_exn order in
              let p1, p2 = if swap then tf_id, other else other, tf_id in
              let n1 = pname p1 in
              let n2 = pname p2 in
              let role = Game_state.character_name state tf_id in
              Botc_exec.tell pid [%string "%{n1} and %{n2} — one is the %{role}"])
          in
          Botc_exec.sleep pid))
;;

let day_action ~player_id:_ = None
let on_setup ~player_id:_ = None
let on_nominated ~player_id:_ ~nominator:_ = None
let on_executed ~player_id:_ = None
let on_night_kill ~player_id:_ = None
