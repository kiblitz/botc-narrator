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
                let all_names =
                  List.map
                    (Game_state.seat_order state)
                    ~f:(Game_state.character_name state)
                in
                let%bind.Botc_exec roles =
                  Botc_exec.narrator_pick
                    "poisoned undertaker role"
                    all_names
                    ~pick_count:1
                in
                let role = List.hd_exn roles in
                Botc_exec.tell pid role)
              else (
                let role = Game_state.character_name state exec_id in
                Botc_exec.tell pid role)
            in
            Botc_exec.sleep pid))
;;

let day_action ~player_id:_ = None
let on_setup ~player_id:_ = None
let on_nominated ~player_id:_ ~nominator:_ = None
let on_executed ~player_id:_ = None
let on_night_kill ~player_id:_ = None
