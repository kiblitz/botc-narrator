open! Core
open Botc_narrator_lib

let make_console_interp (_state : Game_state.t) : (module Botc_exec.Engine_S) =
  let pname id = Player_id.to_string id in
  (module struct
    let wake id = printf "[wake]  %s\n%!" (pname id)
    let sleep id = printf "[sleep] %s\n%!" (pname id)
    let tell id msg = printf "[tell -> %-10s] %s\n%!" (pname id) msg
    let ask _id _question cs = List.hd_exn cs

    let narrator_pick _prompt = function
      | [] -> failwith "narrator_pick: empty"
      | x :: _ -> x
    ;;

    let log s = printf "[log] %s\n%!" s
  end)
;;

let () =
  let mk name (module C : Character_intf.S) =
    { Game_state.Player_spec.id = Player_id.of_string name
    ; character_id = C.id
    ; character_name = C.name
    ; kind = C.kind C.t
    ; alignment = C.alignment C.t
    }
  in
  let specs =
    [ mk "Alice" (module Characters.Imp)
    ; mk "Bob" (module Characters.Poisoner)
    ; mk "Carol" (module Characters.Empath)
    ; mk "Dave" (module Characters.Washerwoman)
    ; mk "Eve" (module Characters.Fortune_teller)
    ; mk "Frank" (module Characters.Chef)
    ]
  in
  let seat_order = List.map specs ~f:(fun (s : Game_state.Player_spec.t) -> s.id) in
  let state = Game_state.create seat_order specs |> Game_state.next_phase in
  let interp = make_console_interp state in
  printf "=== Night 1 ===\n%!";
  let (), _final =
    Botc_exec.run interp state (Narrator.run_night (module Trouble_brewing) ())
  in
  ()
;;
