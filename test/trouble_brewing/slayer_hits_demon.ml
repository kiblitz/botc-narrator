open! Core
open Botc_narrator_lib
open Test_helpers

(* Slayer correctly targets the Imp on day 1, killing it.

   Diablo=Imp  Ivy=Poisoner  Troi=Empath  Gordon=Chef
   Zoltar=Fortune_teller  Friar=Monk  Buffy=Slayer *)

let players =
  mk_players
    [ "Diablo", (module Characters.Imp : Character_intf.S)
    ; "Ivy", (module Characters.Poisoner)
    ; "Troi", (module Characters.Empath)
    ; "Gordon", (module Characters.Chef)
    ; "Zoltar", (module Characters.Fortune_teller)
    ; "Friar", (module Characters.Monk)
    ; "Buffy", (module Characters.Slayer)
    ]
;;

let run = run ~players

let night_1 ?(silent = false) state =
  let run = run ~silent in
  let night_action = night_action ~night:1 in
  state
  |> run ~action:(Narrator.minion_info ())
  |> run ~action:(Narrator.demon_info Trouble_brewing.all ())
  |> run
       ~responses:[ p "Troi" ]
       ~action:(night_action Characters.Poisoner.night_action ~player_id:"Ivy")
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

(* === Tests === *)

let%expect_test "night 1" =
  let state = init () |> night_1 in
  print_grimoire state;
  [%expect
    {|
    narrator->Ivy(Poisoner): wake
    narrator->Ivy(Poisoner): Evil: Diablo, Ivy
    narrator->Ivy(Poisoner): sleep
    narrator->Diablo(Imp): wake
    narrator->Diablo(Imp): Bluffs: Butler, Drunk, Investigator
    narrator->Diablo(Imp): sleep
    narrator->Ivy(Poisoner): wake
    narrator->Ivy(Poisoner): Who do you want to poison?
    Ivy(Poisoner)->narrator: Troi(Empath)
    narrator->Ivy(Poisoner): sleep
    narrator->Gordon(Chef): wake
    narrator->Gordon(Chef): 1 evil pairs
    narrator->Gordon(Chef): sleep
    narrator->Troi(Empath): wake
    narrator->Troi(Empath): 0 evil neighbors
    narrator->Troi(Empath): sleep
    narrator->Zoltar(Fortune Teller): wake
    narrator->Zoltar(Fortune Teller): Choose a player
    Zoltar(Fortune Teller)->narrator: Gordon(Chef)
    narrator->Zoltar(Fortune Teller): Choose a player
    Zoltar(Fortune Teller)->narrator: Friar(Monk)
    narrator->Zoltar(Fortune Teller): No
    narrator->Zoltar(Fortune Teller): sleep
                                         Diablo(Imp)

          Buffy(Slayer)                                               Ivy(Poisoner)




    Friar(Monk)                                                         Troi(Empath) [poisoned]



                   Zoltar(Fortune Teller)               Gordon(Chef)
    |}]
;;

let%expect_test "day 1: slayer kills demon" =
  let state = init () |> night_1 ~silent:true |> day_1 in
  print_grimoire state;
  [%expect
    {|
    narrator->Buffy(Slayer): Who do you want to slay?
    Buffy(Slayer)->narrator: Diablo(Imp)
                                     Diablo(Imp) [dead]

          Buffy(Slayer)                                               Ivy(Poisoner)




    Friar(Monk)                                                         Troi(Empath) [poisoned]



                   Zoltar(Fortune Teller)               Gordon(Chef)
    |}]
;;
