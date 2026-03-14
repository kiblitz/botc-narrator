open! Core

let ( let* ) = Narrator_monad.( let* )

let id        = "empath"
let name      = "Empath"
let kind      = Character_intf.Townsfolk
let alignment = Character_intf.Good

let night_action ~player_id:pid ~night:_ =
  Some (Character.if_alive pid begin
    let* state = Narrator_monad.get_state in
    let* ()    = Narrator_monad.wake pid in
    let* () =
      if Game_state.is_poisoned state pid then begin
        let* n = Narrator_monad.narrator_pick [0; 1; 2] in
        Narrator_monad.show_info pid (Narrator_monad.Number n)
      end else begin
        let alive = Game_state.alive_players state in
        let n     = List.length alive in
        let idx =
          List.findi alive ~f:(fun _ p -> Player_id.equal p.Player.id pid)
          |> Option.value_exn ~message:"empath: player not found"
          |> fst
        in
        let left  = List.nth_exn alive ((idx - 1 + n) mod n) in
        let right = List.nth_exn alive ((idx + 1)     mod n) in
        let count =
          (if Player.is_evil left  then 1 else 0)
          + (if Player.is_evil right then 1 else 0)
        in
        Narrator_monad.show_info pid (Narrator_monad.Number count)
      end
    in
    Narrator_monad.sleep pid
  end)
