open! Core

let ( let* ) = Narrator_monad.( let* )

let id        = "chef"
let name      = "Chef"
let kind      = Character_intf.Townsfolk
let alignment = Character_intf.Good

let night_action ~player_id:pid ~night =
  if night <> 1 then None
  else Some (Character.if_alive pid begin
    let* state = Narrator_monad.get_state in
    let* ()    = Narrator_monad.wake pid in
    let* () =
      if Game_state.is_poisoned state pid then begin
        let* n = Narrator_monad.narrator_pick [0; 1; 2; 3] in
        Narrator_monad.show_info pid (Narrator_monad.Number n)
      end else begin
        let alive = Game_state.alive_players state in
        let n     = List.length alive in
        let count =
          List.foldi alive ~init:0 ~f:(fun i acc p ->
            let next = List.nth_exn alive ((i + 1) mod n) in
            if Player.is_evil p && Player.is_evil next then acc + 1 else acc)
        in
        Narrator_monad.show_info pid (Narrator_monad.Number count)
      end
    in
    Narrator_monad.sleep pid
  end)
