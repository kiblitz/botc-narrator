open! Core
open Botc_narrator_lib
open Test_helpers

(* Slayer kills Imp on day 1 with 7 alive — Scarlet Woman becomes the new demon.
   Night 2 continues with Scarlet Woman as demon (no Imp night action).

   Diablo=Imp  Crimson=Scarlet_woman  Troi=Empath  Gordon=Chef
   Zoltar=Fortune_teller  Friar=Monk  Buffy=Slayer *)

let players =
  mk_players
    [ "Diablo", (module Characters.Imp : Character_intf.S)
    ; "Crimson", (module Characters.Scarlet_woman)
    ; "Troi", (module Characters.Empath)
    ; "Gordon", (module Characters.Chef)
    ; "Zoltar", (module Characters.Fortune_teller)
    ; "Friar", (module Characters.Monk)
    ; "Buffy", (module Characters.Slayer)
    ]
;;

let night_1 ?(silent = false) state =
  let run = run ~silent in
  let night_action = night_action ~night:1 in
  state
  |> run ~action:(Narrator.minion_info ())
  |> run ~action:(Narrator.demon_info Trouble_brewing.all ())
  |> run ~action:(night_action Characters.Chef.night_action ~player_id:"Gordon")
  |> run ~action:(night_action Characters.Empath.night_action ~player_id:"Troi")
  |> run
       ~responses:[ p "Gordon"; p "Friar" ]
       ~action:(night_action Characters.Fortune_teller.night_action ~player_id:"Zoltar")
;;

let day_1 ?(silent = false) state =
  let run = run ~silent in
  Game_state.next_phase state
  |> run
       ~responses:[ p "Diablo" ]
       ~action:(day_action Characters.Slayer.day_action ~player_id:"Buffy")
;;

let init () = night_state players

let%expect_test "day 1: slayer kills imp, scarlet woman promotes" =
  let state = init () |> night_1 ~silent:true |> day_1 in
  print_grimoire state;
  [%expect
    {|
    narrator->Buffy(Slayer): Who do you want to slay?
    Buffy(Slayer)->narrator: Diablo(Imp)
                                     Diablo(Imp) [dead]

          Buffy(Slayer)                                               Crimson(Imp)




    Friar(Monk)                                                              Troi(Empath)



                   Zoltar(Fortune Teller)               Gordon(Chef)
    |}]
;;
