open! Core
open! Import

include Character.Make (struct
    type t =
      { kind : Kind.t
      ; alignment : Alignment.t
      }

    let id = "librarian"
    let name = "Librarian"
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
          let pname id = Player.name (Map.find_exn (Game_state.players state) id) in
          let%bind.Botc_exec () =
            if Game_state.is_poisoned state pid
            then (
              let ids = Game_state.alive_ids state in
              let%bind.Botc_exec p1, rest = narrator_pick_from ids in
              let%bind.Botc_exec p2, _ = narrator_pick_from rest in
              let outsider_chars =
                List.filter_map (Game_state.alive_players state) ~f:(fun p ->
                  if is_outsider (Player.character p)
                  then Some (Player.character p)
                  else None)
              in
              let%bind.Botc_exec char = Botc_exec.narrator_pick outsider_chars in
              let n1 = pname p1 in
              let n2 = pname p2 in
              let role = Char_display.name char in
              Botc_exec.tell pid [%string "%{n1} and %{n2} — one is the %{role}"])
            else (
              let outsiders =
                List.filter (Game_state.alive_players state) ~f:(fun p ->
                  is_outsider (Player.character p))
              in
              if List.is_empty outsiders
              then Botc_exec.tell pid "No outsiders"
              else (
                let%bind.Botc_exec out, _ = narrator_pick_from outsiders in
                let other_ids = alive_except state (Player.id out) in
                let%bind.Botc_exec other, _ = narrator_pick_from other_ids in
                let%bind.Botc_exec swap, _ = narrator_pick_from [ false; true ] in
                let p1, p2 =
                  if swap then Player.id out, other else other, Player.id out
                in
                let n1 = pname p1 in
                let n2 = pname p2 in
                let role = Char_display.name (Player.character out) in
                Botc_exec.tell pid [%string "%{n1} and %{n2} — one is the %{role}"]))
          in
          Botc_exec.sleep pid))
;;

let day_action ~player_id:_ = None
let on_setup ~player_id:_ = None
let on_nominated ~player_id:_ ~nominator:_ = None
let on_executed ~player_id:_ = None
let on_night_kill ~player_id:_ = None
