open! Core
open! Import

include Character.Make (struct
    type t =
      { kind : Kind.t
      ; alignment : Alignment.t
      }

    let id = "spy"
    let name = "Spy"
    let t = { kind = Kind.Minion; alignment = Alignment.Evil }
    let kind { kind; _ } = kind
    let alignment { alignment; _ } = alignment
  end)

let night_action ~player_id:pid ~night:_ =
  Some
    (Character_intf.Read_only
       (if_alive
          pid
          (let%bind.Botc_exec state = Botc_exec.get_state ()
           and () = Botc_exec.wake pid in
           let%bind.Botc_exec () =
             Botc_exec.tell
               pid
               (List.map (Game_state.seat_order state) ~f:(fun id ->
                  let name = Player_id.to_string id in
                  let role = Game_state.character_name state id in
                  [%string "%{name}=%{role}"])
                |> String.concat ~sep:", ")
           in
           Botc_exec.sleep pid)))
;;

let day_action ~player_id:_ = None
let on_setup ~player_id:_ = None
let on_nominated ~player_id:_ ~nominator:_ = None
let on_executed ~player_id:_ = None
let on_night_kill ~player_id:_ = None
