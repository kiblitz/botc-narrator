open! Core
open! Import

include Character.Make (struct
    type t =
      { kind : Kind.t
      ; alignment : Alignment.t
      }

    let id = "undertaker"
    let name = "Undertaker"
    let t = { kind = Kind.Townsfolk; alignment = Alignment.Good }
    let kind { kind; _ } = kind
    let alignment { alignment; _ } = alignment
  end)

let night_action ~player_id:pid ~night =
  if night = 1
  then None
  else
    Some
      (if_alive
         pid
         (let%bind.Botc_exec state = Botc_exec.get_state in
          match Game_state.last_execution state with
          | None -> Botc_exec.return ()
          | Some exec_id ->
            let%bind.Botc_exec () = Botc_exec.wake pid in
            let%bind.Botc_exec () =
              if Game_state.is_poisoned state pid
              then (
                let all_chars =
                  List.map (Game_state.seated_players state) ~f:(fun p ->
                    Player.character p)
                in
                let%bind.Botc_exec char = Botc_exec.narrator_pick all_chars in
                Botc_exec.tell pid (Char_display.name char))
              else (
                let exec_char =
                  Player.character (Map.find_exn (Game_state.players state) exec_id)
                in
                Botc_exec.tell pid (Char_display.name exec_char))
            in
            Botc_exec.sleep pid))
;;

let day_action ~player_id:_ = None
let on_setup ~player_id:_ = None
let on_nominated ~player_id:_ ~nominator:_ = None
let on_executed ~player_id:_ = None
let on_night_kill ~player_id:_ = None
