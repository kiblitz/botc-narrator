open! Core
open! Import

include Character.Make (struct
    type t =
      { kind : Kind.t
      ; alignment : Alignment.t
      }

    let id = "ravenkeeper"
    let name = "Ravenkeeper"
    let t = { kind = Kind.Townsfolk; alignment = Alignment.Good }
    let kind { kind; _ } = kind
    let alignment { alignment; _ } = alignment
  end)

let night_action ~player_id:pid ~night =
  if night = 1
  then None
  else
    Some
      (let%bind.Botc_exec state = Botc_exec.get_state in
       let is_dead = not (Game_state.is_alive state pid) in
       if not is_dead
       then Botc_exec.return ()
       else (
         let%bind.Botc_exec () = Botc_exec.wake pid in
         let candidates = Game_state.alive_ids state in
         let%bind.Botc_exec target =
           Botc_exec.ask pid "Who do you want to learn about?" candidates
         in
         let%bind.Botc_exec () =
           if Game_state.is_poisoned state pid
           then (
             let all_names =
               List.map (Game_state.seat_order state) ~f:(Game_state.character_name state)
             in
             let%bind.Botc_exec roles =
               Botc_exec.narrator_pick "poisoned ravenkeeper role" all_names ~pick_count:1
             in
             let role = List.hd_exn roles in
             Botc_exec.tell pid role)
           else (
             let role = Game_state.character_name state target in
             Botc_exec.tell pid role)
         in
         Botc_exec.sleep pid))
;;

let day_action ~player_id:_ = None
let on_setup ~player_id:_ = None
let on_nominated ~player_id:_ ~nominator:_ = None
let on_executed ~player_id:_ = None
let on_night_kill ~player_id:_ = None
