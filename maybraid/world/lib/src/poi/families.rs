//! Which world [`PoiOrigin`] families can satisfy an interest set.

use mob_characters::{LOCAL_POI, SALOON_POI, URBAN_POI, VEGETATION_POI};
use poi_intelligence::PoiInterests;

pub(crate) fn interests_need_grove(interests: &PoiInterests) -> bool {
	interests.contains(VEGETATION_POI)
}

pub(crate) fn interests_need_urban(interests: &PoiInterests) -> bool {
	interests.contains(URBAN_POI)
}

pub(crate) fn interests_need_places(interests: &PoiInterests) -> bool {
	interests.contains(LOCAL_POI) || interests.contains(SALOON_POI)
}
