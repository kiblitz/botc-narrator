open! Core

val minion_info : unit -> (unit, _) Botc_exec.t
val demon_info : (module Character_intf.S) list -> unit -> (unit, _) Botc_exec.t
val run_order : (module Character_intf.S) list -> int -> (unit, Botc_exec.rw) Botc_exec.t
val run_night : (module Script_intf.S) -> unit -> (unit, Botc_exec.rw) Botc_exec.t
