use crate::state::{Player, PLAYERS};
use crate::ContractError;
use cosmwasm_std::{CanonicalAddr, Storage, Uint128};

pub fn update_player(
    storage: &mut dyn Storage,
    raw_address: &CanonicalAddr,
    game_rewards: Uint128,
    won: bool,
    is_profit: Option<bool>,
) -> Result<(), ContractError> {
    match PLAYERS.may_load(storage, &raw_address.as_slice())? {
        None => {
            let calculation_rewards = if let Some(profit) = is_profit {
                if profit && !game_rewards.is_zero() {
                    game_rewards
                } else {
                    Uint128::zero()
                }
            } else {
                Uint128::zero()
            };

            let player = if won {
                Player {
                    game_won: 1,
                    game_over: 0,
                    game_rewards: calculation_rewards,
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

                    if let Some(profit) = is_profit {
                        if !game_rewards.is_zero() {
                            if profit {
                                update_player.game_rewards = update_player
                                    .game_rewards
                                    .checked_add(game_rewards)
                                    .unwrap();
                            } else {
                                if update_player.game_rewards > game_rewards {
                                    update_player.game_rewards = update_player
                                        .game_rewards
                                        .checked_sub(game_rewards)
                                        .unwrap();
                                } else {
                                    update_player.game_rewards = Uint128::zero();
                                }
                            }
                        }
                    }

                    if won {
                        update_player.game_won += 1;
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
