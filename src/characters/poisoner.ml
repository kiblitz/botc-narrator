open! Core

let ( let* ) = Narrator_monad.( let* )

let id        = "poisoner"
let name      = "Poisoner"
let kind      = Character_intf.Minion
let alignment = Character_intf.Evil

let night_action ~player_id:pid ~night:_ =
  Some (Character.if_alive pid begin
    let* state  = Narrator_monad.get_state in
    let* ()     = Narrator_monad.wake pid in
    let* target = Narrator_monad.player_points pid (Character.alive_except state pid) in
    let* ()     = Narrator_monad.modify_state (fun s -> Game_state.set_poisoned s target) in
    Narrator_monad.sleep pid
  end)
