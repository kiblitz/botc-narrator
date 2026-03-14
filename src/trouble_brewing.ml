open! Core

let ( let* ) = Narrator_monad.( let* )
let ( >>=   ) = Narrator_monad.( >>= )
let ( >>    ) = Narrator_monad.( >>  )

let night_one_order : (module Character.WITH_NIGHT_ACTION) list = [
  (module Poisoner);
  (module Washerwoman);
  (module Librarian);
  (module Investigator);
  (module Chef);
  (module Empath);
  (module Fortune_teller);
  (module Butler);
]

let night_subsequent_order : (module Character.WITH_NIGHT_ACTION) list = [
  (module Poisoner);
  (module Monk);
  (module Imp);
  (module Ravenkeeper);
  (module Undertaker);
  (module Spy);
  (module Empath);
  (module Fortune_teller);
  (module Butler);
]

let run_order order night =
  List.fold order ~init:(Narrator_monad.return ()) ~f:(fun acc (module C : Character.WITH_NIGHT_ACTION) ->
    acc >>= fun () ->
    let* state = Narrator_monad.get_state in
    match Game_state.find_character_id state C.id with
    | None        -> Narrator_monad.return ()
    | Some player ->
      match C.night_action ~player_id:player.Player.id ~night with
      | None   -> Narrator_monad.return ()
      | Some m -> m)

let run_night_one () =
  Characters.minion_info ()
  >> Characters.demon_info ()
  >> run_order night_one_order 1

let run_night_subsequent n =
  run_order night_subsequent_order n

let run_night () =
  let* state = Narrator_monad.get_state in
  match state.Game_state.phase with
  | Game_state.Night { number = 1 } -> run_night_one ()
  | Game_state.Night { number = n } -> run_night_subsequent n
  | _                               -> Narrator_monad.return ()
