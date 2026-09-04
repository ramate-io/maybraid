# Mobs

- [`./characters`](./characters): characters define mob NPCs and noisy builders thereof. They include a species, an inventory, and a set of brains. We define four sets of noisy constructors: 
    - **build:** base attribute adjustments for the character. 
    - **species:** defines a species. 
    - **inventory:** generates an appropriate inventory. 
    - **brains:** defines a set of brains within a class hierarchy. 

The user will then access a constructor `MobCharacter<Build, Species, Inventory, Brains>::from_num(num: f32)` that gives the character. 


- [`./mobs`](./mobs): mobs compose rosters of characters and define mob intelligence brains. Hence we typically have `Mob<Roster, Intelligence>`.
- [`./groups`](./groups): groups compose multiple mobs for a supposed mob cell. This is similar to forest cells where mobs are the groves. They differ from forest cells in that they map urbanization dependencies to form selection criteria. 

For this version, we propose to take a direct gen model on available grove and urbanization types from `chico` and `richmond`. This will allow us to work out some initial rules before trying to take a step back and generalize. 

Spawning APIs from groups targeting the Richmond Surface model `Arc<RichmonSurfaceModel>` (forgot if this is the actual name) to determine where reasonable spawn points are. We will likely make this its own crate. I'm not sure if we want to bake the Richmond type in here so that we have the right respawn brain, or if we author a patch system that will apply/make available the Richmond spawn points model at runtime. 

Groups may put multiple mobs over the same area. We typically use large `400m` cells and allocate 2-12 mobs per cell. We may even use multiple groups over the same POI initialization to create a more dynamic feel. 

## Characters
Initial proposed characters are as follows:

### Build

- **Base:** base build.
- **Wraith:** generous buff to speed. Reduction to health.
- **Tank:** generous buff to health. Reduction to speed.
- **Brawler:** generous buff to strength. Reduction to agility.
- **Renegade:** small buffs to damage and speed.
- **Warrior:** small buffs to all stats. 
- **Master:** generous buffs to all stats.

### Species
All species except the fish species are included. 

### Inventory

- **Empty:** no inventory; used for animals. All quadrupeds get this and no other inventory. 
- **Clothed:** a set of clothing items. 
- **Flashy:** a set of rare clothing items.
- **Grunt:** a set of common clothing items and a generic weapon. 
- **Mercenary:** a set of rare clothing items and a rare but generic weapon. 
- **Specialist:** a set of rare clothing items and a rare weapon of a specific type. 
- **Mist:** a set of rare clothing items and a very rare weapon of a specific type. 

## Brains

- **Grazinger:** mostly for quadrupeds, has vegetation local POI. Varied combat skills.
- **Pack Hunter:** has other character local POI (like herd members). Mostly combatitive; sensitive to low health for fleeing. Varied combat skills.
- **Raider:** mostly for bipeds, has POI for other characters (mostly civilians and other raider groups) and town or urban areas. Varied combat skills. 
- **Guard:** bipeds or quadrupeds. Combatitive; sensitive to low health for fleeing. Varied combat skills but rarely low. 
- **Civilian:** bipeds or quadrupeds. Local POI. No long-range movements. Rarely combatitive; mostly fleeing. Low combat skills.
- **Roamer:** general exploring type. Local POI. Varied combat skills.
- **Brawler:** replicates FFA behavior in real world. Creates an FFA saloon. Varied combat skills.

## Mobs

- **Herd:** 1-24 characters. Grazingers and maybe a Roamer or two. Mixed species within. Preference for herbivorous quadrupeds.  
- **Pack:** 3-12 characters. Pack Hunters. One species. Preference for carnivorous quadrupeds.
- **Raider:** 3-12 characters. Raiders and maybe a Roamer. Mixed bipeds within. Preference for bipeds.
- **Guard:** 3-12 characters. Guards and maybe a Roamer. Mixed bipeds or quadrupeds within. Preference for bipeds or quadrupeds. Tending to have a most common species. 
- **Pleb:** 10-24 characters. Civilians and maybe a Roamer.
- **Rambles:** 1-12 characters. Roamers. Mixed species within. Prefence for either quadruped or biped as opposed to mixing across both. 
- **Brawler:** 6-12 characters. Brawlers. Mixed species within. Bipeds. 

## Groups

- **Peaceful:** 2-12 mobs. Prioritize allocatig **Pleb** to urbanization triggers and **Herd** to vegetation triggers. 
- **Wild:** 2-8 mobs. Allocate mix of **Raider**, **Herd**, **Pack**, and **Rambler** to form a balanced group. 
- **Frontier:** 2-12 mobs. Prioritize **Guard**, **Raider**, and **Brawler** on urbanization triggers and **Herd** and **Pack** on vegetation triggers. 
- **Warfront:** 2-12 mobs. Prioritize **Guard**, **Raider**, **Plebs**, and **Brawler** on urbanization triggers and **Herd** and **Pack** on vegetation triggers. 
- **Dystopian:** 2-12 mobs. Prioritize **Guard** and **Pleb** on urbanization triggers and **Herd** on vegetation triggers. 