open! Core

let ( let* ) = Narrator_monad.( let* )

let id        = "fortune_teller"
let name      = "Fortune Teller"
let kind      = Character_intf.Townsfolk
let alignment = Character_intf.Good

let night_action ~player_id:pid ~night:_ =
  Some (Character.if_alive pid begin
    let* state     = Narrator_monad.get_state in
    let* ()        = Narrator_monad.wake pid in
    let candidates = Game_state.alive_ids state in
    let* (p1, p2)  = Narrator_monad.player_points_two pid candidates in
    let result =
      if Game_state.is_poisoned state pid then false
      else
        let char_of id = (Map.find_exn state.Game_state.players id).Player.character in
        Character_intf.is_demon (char_of p1) || Character_intf.is_demon (char_of p2)
    in
    let* () = Narrator_monad.show_info pid (Narrator_monad.Yes_or_no result) in
    Narrator_monad.sleep pid
  end)
