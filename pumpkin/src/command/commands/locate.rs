use crate::command::args::resource_location::ResourceLocationArgumentConsumer;
use crate::command::args::{ConsumedArgs, FindArgDefaultName};
use crate::command::dispatcher::CommandError;
use crate::command::tree::builder::argument_default_name;
use crate::command::tree::CommandTree;
use crate::command::{CommandExecutor, CommandResult, CommandSender};
use crate::server::Server;
use pumpkin_data::structures::{StructureKeys, StructurePlacement, StructureSet};
use pumpkin_data::translation;
use pumpkin_util::text::TextComponent;
use pumpkin_world::generation::generator::structure_finder::find_nearest_structure;

const DESCRIPTION: &str = "Locates the nearest structure of a given type.";
const MAX_SEARCH_RADIUS_CHUNKS: i32 = 100;

struct LocateStructureExecutor;

fn parse_structure_key(name: &str) -> Option<StructureKeys> {
    let key = name.strip_prefix("minecraft:").unwrap_or(name);

    match key {
        "pillager_outpost" => Some(StructureKeys::PillagerOutpost),
        "mineshaft_mesa" => Some(StructureKeys::MineshaftMesa),
        "mansion" | "woodland_mansion" => Some(StructureKeys::Mansion),
        "jungle_pyramid" | "jungle_temple" => Some(StructureKeys::JunglePyramid),
        "desert_pyramid" => Some(StructureKeys::DesertPyramid),
        "igloo" => Some(StructureKeys::Igloo),
        "shipwreck_beached" => Some(StructureKeys::ShipwreckBeached),
        "swamp_hut" | "witch_hut" => Some(StructureKeys::SwampHut),
        "stronghold" => Some(StructureKeys::Stronghold),
        "monument" | "ocean_monument" => Some(StructureKeys::Monument),
        "ocean_ruin_cold" => Some(StructureKeys::OceanRuinCold),
        "ocean_ruin_warm" => Some(StructureKeys::OceanRuinWarm),
        "fortress" | "nether_fortress" => Some(StructureKeys::Fortress),
        "nether_fossil" => Some(StructureKeys::NetherFossil),
        "end_city" => Some(StructureKeys::EndCity),
        "buried_treasure" => Some(StructureKeys::BuriedTreasure),
        "bastion_remnant" => Some(StructureKeys::BastionRemnant),
        "village_plains" => Some(StructureKeys::VillagePlains),
        "village_desert" => Some(StructureKeys::VillageDesert),
        "village_savanna" => Some(StructureKeys::VillageSavanna),
        "village_snowy" => Some(StructureKeys::VillageSnowy),
        "village_taiga" => Some(StructureKeys::VillageTaiga),
        "ruined_portal_desert" => Some(StructureKeys::RuinedPortalDesert),
        "ruined_portal_jungle" => Some(StructureKeys::RuinedPortalJungle),
        "ruined_portal_swamp" => Some(StructureKeys::RuinedPortalSwamp),
        "ruined_portal_mountain" => Some(StructureKeys::RuinedPortalMountain),
        "ruined_portal_ocean" => Some(StructureKeys::RuinedPortalOcean),
        "ruined_portal_nether" => Some(StructureKeys::RuinedPortalNether),
        "ancient_city" => Some(StructureKeys::AncientCity),
        "trail_ruins" => Some(StructureKeys::TrailRuins),
        "trial_chambers" => Some(StructureKeys::TrialChambers),
        _ => None,
    }
}

fn resolve_structure_placements(name: &str) -> Option<Vec<&'static StructurePlacement>> {
    let key = name.strip_prefix("minecraft:").unwrap_or(name);

    if let Some(set) = StructureSet::get(key) {
        return Some(vec![&set.placement]);
    }

    match key {
        "village" | "villages" => return Some(vec![&StructureSet::VILLAGES.placement]),
        "ocean_ruin" | "ocean_ruins" => return Some(vec![&StructureSet::OCEAN_RUINS.placement]),
        "ruined_portal" | "ruined_portals" => {
            return Some(vec![&StructureSet::RUINED_PORTALS.placement]);
        }
        "mineshaft" | "mineshafts" => return Some(vec![&StructureSet::MINESHAFTS.placement]),
        "shipwreck" | "shipwrecks" => return Some(vec![&StructureSet::SHIPWRECKS.placement]),
        "nether_complex" | "nether_complexes" => {
            return Some(vec![&StructureSet::NETHER_COMPLEXES.placement]);
        }
        _ => {}
    }

    let structure_key = parse_structure_key(key)?;
    let placements: Vec<&StructurePlacement> = StructureSet::ALL
        .iter()
        .filter(|set| {
            set.structures
                .iter()
                .any(|entry| entry.structure == structure_key)
        })
        .map(|set| &set.placement)
        .collect();

    (!placements.is_empty()).then_some(placements)
}

impl CommandExecutor for LocateStructureExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        Box::pin(async move {
            let structure_name = ResourceLocationArgumentConsumer.find_arg_default_name(args)?;
            let structure_name = structure_name.to_string();

            let placements = resolve_structure_placements(&structure_name).ok_or_else(|| {
                CommandError::CommandFailed(TextComponent::translate_cross(
                    translation::java::COMMANDS_LOCATE_STRUCTURE_INVALID,
                    translation::java::COMMANDS_LOCATE_STRUCTURE_INVALID,
                    [TextComponent::text(structure_name.clone())],
                ))
            })?;

            let player = sender.as_player().ok_or_else(|| {
                CommandError::CommandFailed(TextComponent::translate_cross(
                    translation::bedrock::COMMANDS_LOCATE_STRUCTURE_FAIL_NOPLAYER,
                    translation::bedrock::COMMANDS_LOCATE_STRUCTURE_FAIL_NOPLAYER,
                    [],
                ))
            })?;

            let world = player.world();
            let origin = player.position().to_block_pos();
            let generator = &world.level.world_gen;
            let seed = world.level.seed.0;

            let target = find_nearest_structure(
                origin,
                &placements,
                MAX_SEARCH_RADIUS_CHUNKS,
                seed as i64,
                &generator.global_structure_cache,
            );

            if let Some(target_pos) = target {
                let squared_distance = origin.0.squared_distance_to_vec(&target_pos.0);
                let distance = (squared_distance as f64).sqrt().round() as i32;
                let position = format!("[{}, ~, {}]", target_pos.0.x, target_pos.0.z);

                sender
                    .send_message(TextComponent::translate_cross(
                        translation::java::COMMANDS_LOCATE_STRUCTURE_SUCCESS,
                        translation::java::COMMANDS_LOCATE_STRUCTURE_SUCCESS,
                        [
                            TextComponent::text(structure_name),
                            TextComponent::text(position),
                            TextComponent::text(distance.to_string()),
                        ],
                    ))
                    .await;

                Ok(1)
            } else {
                Err(CommandError::CommandFailed(TextComponent::translate_cross(
                    translation::java::COMMANDS_LOCATE_STRUCTURE_NOT_FOUND,
                    translation::java::COMMANDS_LOCATE_STRUCTURE_NOT_FOUND,
                    [TextComponent::text(structure_name)],
                )))
            }
        })
    }
}

pub fn init_command_tree() -> CommandTree {
    CommandTree::new(["locate"], DESCRIPTION).then(
        crate::command::tree::builder::literal("structure")
            .then(argument_default_name(ResourceLocationArgumentConsumer).execute(LocateStructureExecutor)),
    )
}
