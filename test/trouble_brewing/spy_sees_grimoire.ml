open! Core
open Botc_narrator_lib
open Test_helpers

(* Spy wakes on night 2 and sees the full grimoire.

   Diablo=Imp  Snooper=Spy  Troi=Empath  Gordon=Chef
   Zoltar=Fortune_teller  Friar=Monk  Buffy=Slayer *)

let players =
  mk_players
    [ "Diablo", (module Characters.Imp : Character_intf.S)
    ; "Snooper", (module Characters.Spy)
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
       ~responses:[ p "Gordon"; p "Buffy" ]
       ~action:(night_action Characters.Fortune_teller.night_action ~player_id:"Zoltar")
;;

let night_2 ?(silent = false) state =
  let run = run ~silent in
  let night_action = night_action ~night:2 in
  Game_state.next_phase state
  |> run
       ~responses:[ p "Gordon" ]
       ~action:(night_action Characters.Monk.night_action ~player_id:"Friar")
  |> run
       ~responses:[ p "Troi" ]
       ~action:(night_action Characters.Imp.night_action ~player_id:"Diablo")
  |> run ~action:(night_action Characters.Spy.night_action ~player_id:"Snooper")
  |> run ~action:(night_action Characters.Empath.night_action ~player_id:"Troi")
  |> run
       ~responses:[ p "Diablo"; p "Snooper" ]
       ~action:(night_action Characters.Fortune_teller.night_action ~player_id:"Zoltar")
;;

let init () = night_state players

let%expect_test "night 2: spy sees full grimoire" =
  let state =
    init () |> night_1 ~silent:true |> fun s -> Game_state.next_phase s |> night_2
  in
  print_grimoire state;
  [%expect
    {|
    narrator->Friar(Monk): wake
    narrator->Friar(Monk): Who do you want to protect?
    Friar(Monk)->narrator: Gordon(Chef)
    narrator->Friar(Monk): sleep
    narrator->Diablo(Imp): wake
    narrator->Diablo(Imp): Who do you want to kill?
    Diablo(Imp)->narrator: Troi(Empath)
    narrator->Diablo(Imp): sleep
    narrator->Snooper(Spy): wake
    narrator->Snooper(Spy): Diablo=Imp, Snooper=Spy, Troi=Empath, Gordon=Chef, Zoltar=Fortune Teller, Friar=Monk, Buffy=Slayer
    narrator->Snooper(Spy): sleep
    narrator->Zoltar(Fortune Teller): wake
    narrator->Zoltar(Fortune Teller): Choose a player
    Zoltar(Fortune Teller)->narrator: Diablo(Imp)
    narrator->Zoltar(Fortune Teller): Choose a player
    Zoltar(Fortune Teller)->narrator: Snooper(Spy)
    narrator->Zoltar(Fortune Teller): Yes
    narrator->Zoltar(Fortune Teller): sleep
                                         Diablo(Imp)

          Buffy(Slayer)                                               Snooper(Spy)




    Friar(Monk)                                                           Troi(Empath) [dead]



                   Zoltar(Fortune Teller)               Gordon(Chef)
    |}]
;;
