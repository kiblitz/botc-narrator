open! Core

let ( let* ) = Narrator_monad.( let* )

let id        = "monk"
let name      = "Monk"
let kind      = Character_intf.Townsfolk
let alignment = Character_intf.Good

let night_action ~player_id:pid ~night =
  if night = 1 then None
  else Some (Character.if_alive pid begin
    let* state     = Narrator_monad.get_state in
    let* ()        = Narrator_monad.wake pid in
    let candidates = Character.alive_except state pid in
    let* target    = Narrator_monad.player_points pid candidates in
    let* ()        = Narrator_monad.modify_state (fun s -> Game_state.set_monk_protected s target) in
    Narrator_monad.sleep pid
  end)
