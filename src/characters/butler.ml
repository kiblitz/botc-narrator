open! Core

let ( let* ) = Narrator_monad.( let* )

let id        = "butler"
let name      = "Butler"
let kind      = Character_intf.Outsider
let alignment = Character_intf.Good

let night_action ~player_id:pid ~night:_ =
  Some (Character.if_alive pid begin
    let* state     = Narrator_monad.get_state in
    let* ()        = Narrator_monad.wake pid in
    let candidates = Character.alive_except state pid in
    let* _master   = Narrator_monad.player_points pid candidates in
    Narrator_monad.sleep pid
  end)
