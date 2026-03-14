open! Core

let ( let* ) = Narrator_monad.( let* )

let id        = "spy"
let name      = "Spy"
let kind      = Character_intf.Minion
let alignment = Character_intf.Evil

let night_action ~player_id:pid ~night:_ =
  Some (Character.if_alive pid begin
    let* state = Narrator_monad.get_state in
    let* ()    = Narrator_monad.wake pid in
    let grimoire =
      List.map (Game_state.seated_players state)
        ~f:(fun p -> (p.Player.id, Character_intf.name p.Player.character))
    in
    let* () = Narrator_monad.show_info pid (Narrator_monad.Grimoire grimoire) in
    Narrator_monad.sleep pid
  end)
