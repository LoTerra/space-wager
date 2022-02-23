use crate::state::{Player, PLAYERS};
use crate::ContractError;
use cosmwasm_std::{CanonicalAddr, Storage, Uint128};

pub fn update_player(
    storage: &mut dyn Storage,
    raw_address: &CanonicalAddr,
    game_rewards: Uint128,
) -> Result<(), ContractError> {
    match PLAYERS.may_load(storage, &raw_address.as_slice())? {
        None => {
            let player = if !game_rewards.is_zero() {
                Player {
                    game_won: 1,
                    game_over: 0,
                    game_rewards,
                }
            } else {
                Player {
                    game_won: 0,
                    game_over: 1,
                    game_rewards: Uint128::zero(),
                }
            };

            PLAYERS.save(storage, &raw_address.as_slice(), &player)?;
        }
        Some(_) => {
            PLAYERS.update(
                storage,
                &raw_address.as_slice(),
                |player| -> Result<_, ContractError> {
                    let mut update_player = player.unwrap();

                    if !game_rewards.is_zero() {
                        update_player.game_won += 1;
                        update_player.game_rewards = update_player
                            .game_rewards
                            .checked_add(game_rewards)
                            .unwrap();
                    } else {
                        update_player.game_over += 1;
                    }
                    Ok(update_player)
                },
            )?;
        }
    };
    Ok(())
}
