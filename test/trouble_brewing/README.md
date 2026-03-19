# Trouble Brewing Test Scenarios

Each file is a self-contained game walkthrough exercising different character interactions.

| File | Scenario |
|---|---|
| `poisoned_chef.ml` | Poisoner targets Chef night 1, giving wrong pair count. Slayer misses on day 1. Night 2: Poisoner retargets Fortune Teller (wrong reads), Monk protects Empath, Imp kills Chef. |
| `slayer_hits_demon.ml` | Slayer correctly identifies and kills the Imp on day 1. Chef sees 1 evil pair (Imp+Poisoner adjacent). Poisoner had targeted Empath giving wrong info. |
| `monk_saves_target.ml` | Monk protects Fortune Teller on night 2 — Imp targets the same player but the kill is blocked. Nobody dies. |
| `soldier_survives.ml` | Imp targets the Soldier on night 2 — Soldier's passive ability prevents the kill. No deaths occur. |
| `imp_starpass.ml` | Imp kills itself on night 2, passing the demon role to the Poisoner (starpass). Grimoire shows Ivy promoted from Poisoner to Imp. |
| `virgin_nomination.ml` | A Townsfolk (Chef) nominates the Virgin on day 1 and is executed by the Virgin's ability. |
| `ravenkeeper_death.ml` | Imp kills the Ravenkeeper on night 2. Ravenkeeper wakes after death and correctly identifies the Poisoner's role. |
| `spy_sees_grimoire.ml` | Spy wakes on night 2 and sees every player's true character. Imp kills Empath the same night. |
| `scarlet_woman_promotion.ml` | Slayer kills the Imp on day 1 with 7 players alive. Scarlet Woman automatically promotes to become the new Imp. |
