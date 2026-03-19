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
       let died_tonight =
         List.mem (Game_state.night_deaths state) pid ~equal:Player_id.equal
       in
       if not died_tonight
       then Botc_exec.return ()
       else (
         let%bind.Botc_exec () = Botc_exec.wake pid in
         let%bind.Botc_exec target =
           Botc_exec.ask
             pid
             "Who do you want to learn about?"
             (Game_state.alive_ids state)
         in
         let%bind.Botc_exec () =
           if Game_state.is_poisoned state pid
           then (
             let all_chars =
               List.map (Game_state.seated_players state) ~f:(fun p -> Player.character p)
             in
             let%bind.Botc_exec char = Botc_exec.narrator_pick all_chars in
             Botc_exec.tell pid (Char_display.name char))
           else (
             let char =
               Player.character (Map.find_exn (Game_state.players state) target)
             in
             Botc_exec.tell pid (Char_display.name char))
         in
         Botc_exec.sleep pid))
;;

let day_action ~player_id:_ = None
let on_setup ~player_id:_ = None
let on_nominated ~player_id:_ ~nominator:_ = None
let on_executed ~player_id:_ = None
let on_night_kill ~player_id:_ = None
