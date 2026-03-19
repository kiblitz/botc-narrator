open! Core
open Botc_narrator_lib
open Test_helpers

(* Monk protects the Imp's target on night 2 — no death occurs.

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

let night_1 ?(silent = false) state =
  let run = run ~silent in
  let night_action = night_action ~night:1 in
  state
  |> run ~action:(Narrator.minion_info ())
  |> run ~action:(Narrator.demon_info Trouble_brewing.all ())
  |> run
       ~responses:[ p "Buffy" ]
       ~action:(night_action Characters.Poisoner.night_action ~player_id:"Ivy")
  |> run ~action:(night_action Characters.Chef.night_action ~player_id:"Gordon")
  |> run ~action:(night_action Characters.Empath.night_action ~player_id:"Troi")
  |> run
       ~responses:[ p "Gordon"; p "Friar" ]
       ~action:(night_action Characters.Fortune_teller.night_action ~player_id:"Zoltar")
;;

let night_2 ?(silent = false) state =
  let run = run ~silent in
  let night_action = night_action ~night:2 in
  Game_state.next_phase state
  |> run
       ~responses:[ p "Gordon" ]
       ~action:(night_action Characters.Poisoner.night_action ~player_id:"Ivy")
  |> run
       ~responses:[ p "Zoltar" ]
       ~action:(night_action Characters.Monk.night_action ~player_id:"Friar")
  |> run
       ~responses:[ p "Zoltar" ]
       ~action:(night_action Characters.Imp.night_action ~player_id:"Diablo")
  |> run ~action:(night_action Characters.Empath.night_action ~player_id:"Troi")
  |> run
       ~responses:[ p "Diablo"; p "Ivy" ]
       ~action:(night_action Characters.Fortune_teller.night_action ~player_id:"Zoltar")
;;

let init () = night_state players

let%expect_test "night 2: monk protects imp's target" =
  let state =
    init () |> night_1 ~silent:true |> fun s -> Game_state.next_phase s |> night_2
  in
  print_grimoire state;
  [%expect
    {|
    narrator->Ivy(Poisoner): wake
    narrator->Ivy(Poisoner): Who do you want to poison?
    Ivy(Poisoner)->narrator: Gordon(Chef)
    narrator->Ivy(Poisoner): sleep
    narrator->Friar(Monk): wake
    narrator->Friar(Monk): Who do you want to protect?
    Friar(Monk)->narrator: Zoltar(Fortune Teller)
    narrator->Friar(Monk): sleep
    narrator->Diablo(Imp): wake
    narrator->Diablo(Imp): Who do you want to kill?
    Diablo(Imp)->narrator: Zoltar(Fortune Teller)
    narrator->Diablo(Imp): sleep
    narrator->Troi(Empath): wake
    narrator->Troi(Empath): 1 evil neighbors
    narrator->Troi(Empath): sleep
    narrator->Zoltar(Fortune Teller): wake
    narrator->Zoltar(Fortune Teller): Choose a player
    Zoltar(Fortune Teller)->narrator: Diablo(Imp)
    narrator->Zoltar(Fortune Teller): Choose a player
    Zoltar(Fortune Teller)->narrator: Ivy(Poisoner)
    narrator->Zoltar(Fortune Teller): Yes
    narrator->Zoltar(Fortune Teller): sleep
                                         Diablo(Imp)

          Buffy(Slayer)                                               Ivy(Poisoner)




    Friar(Monk)                                                              Troi(Empath)



                   Zoltar(Fortune Teller)          Gordon(Chef) [poisoned]
    |}]
;;
