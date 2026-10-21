# Amnesiac abilities: starter set (draft)

A draft list of strong ("busted") Amnesiac abilities for the simulator, for
the group to edit before any are implemented. Each is written as the
storyteller would hand it out, with the building blocks it would compose
(see `docs/adding-characters.md`). Strike, edit or add lines freely.

## Information

1. **Each night, choose a player: learn their character.** (choose-player, learn-character)
2. **Each night, learn which alive player is the Demon, but you are drunk on even nights.** (learn-player, malfunction-schedule)
3. **You start knowing every Minion in play (players, not characters).** (first-night, learn-players)
4. **Each night, choose 2 players: learn how many of them are evil.** (choose-players, learn-number)
5. **Each day, privately ask the Storyteller a yes/no question: they answer truthfully.** (day-ability, storyteller-answer)
6. **Each night, learn a player and their character; they are always good.** (learn-player, learn-character)
7. **When a player dies, you learn their character.** (on-death, learn-character)

## Protection

8. **Each night, choose a player (not yourself): they cannot die tonight.** (choose-player, protect-all-causes)
9. **You cannot die.** (intercept-death)
10. **The first time the Demon would kill a good player, you die instead.** (intercept-death, redirect)
11. **Good players can't be executed while you live.** (intercept-execution)

## Killing

12. **Once per game, at night, choose a player: they die.** (once-per-game, choose-player, kill)
13. **Each night*, choose a player: if they are a Minion, they die.** (choose-player, kill-if-kind)
14. **If you are executed, the Demon dies too.** (on-executed, kill-demon)
15. **Once per game, during the day, publicly choose a player: if they are evil, they die.** (day-ability, kill-if-evil)

## Rule-changers

16. **Each night, choose a player: they are sober & healthy tonight and tomorrow.** (choose-player, sober)
17. **Once per game, at night, choose a player: they become a good Townsfolk of your choice.** (once-per-game, transform)
18. **Your vote counts as 3.** (vote-weight)
19. **Once per game, during the day, publicly choose a dead player: they come back to life.** (day-ability, revive)
20. **If the Demon nominates you, they are executed immediately.** (on-nominated, execute)

## Open questions

- Should the guess game stay (daily guess, hot/warm/cold), or does the group
  play with the ability simply revealed?
- Should the storyteller pick from this list at random, or by a weight
  ("busted" ones rarer)?
