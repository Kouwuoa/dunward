use super::MegabufferId;
use super::region::{AllocatedMegabufferRegion, FreeMegabufferRegion};
use color_eyre::eyre::{Result, eyre};

/// Mutex-guarded CPU state of the [`Megabuffer`]
pub(crate) struct MegabufferFreeList {
    megabuffer_id: MegabufferId,
    free_regions: Vec<FreeMegabufferRegion>,
}

impl MegabufferFreeList {
    /// Initialize a single free region covering the entire capacity
    pub fn new(megabuffer_id: MegabufferId, total_capacity: u64) -> Self {
        Self {
            megabuffer_id,
            free_regions: vec![FreeMegabufferRegion {
                offset: 0,
                size: total_capacity,
            }],
        }
    }

    /// Find a free region that can fit the allocation and splits it into 2 free regions if possible
    /// # Returns
    /// * `Some(FreeMegabufferRegion)` one free region out of the 2 new split free regions that can fit the allocation
    /// * `None` if no free region can fit the allocation
    pub fn carve_free_region(&mut self, alloc_size: u64) -> Option<FreeMegabufferRegion> {
        let (region_index, new_region) = self
            .free_regions
            .iter_mut()
            .enumerate()
            // Find the first free region that can fit the allocation
            .find(|(_, region)| region.size >= alloc_size)
            .map(|(i, region)| {
                // Split the free region into 2 regions:
                // 1. A free region that fits the allocation exactly
                // 2. The remaining free region
                let offset = region.offset;
                region.offset += alloc_size;
                region.size -= alloc_size;
                (
                    // Index of the remaining free region
                    i,
                    // The free region that fits the allocation exactly,
                    // ready to be inserted into the free regions vector
                    FreeMegabufferRegion {
                        offset,
                        size: alloc_size,
                    },
                )
            })?;

        // Insert the new free region into the free regions vector
        if self.free_regions[region_index].size == 0 {
            self.free_regions[region_index] = new_region.clone();
        } else {
            self.free_regions.insert(region_index, new_region.clone());
        }

        Some(new_region)
    }

    pub fn reclaim_region(&mut self, region: &mut AllocatedMegabufferRegion) -> Result<()> {
        if !region.belongs_to_megabuffer_id(self.megabuffer_id) {
            return Err(eyre!(
                "Attempted to reclaim a region that does not belong to this megabuffer"
            ));
        }

        self.reclaim(region.offset, region.size);
        region.size = 0; // Mark the region as invalid by setting size to 0

        Ok(())
    }

    pub fn defragment_free_regions(&mut self) {
        self.free_regions.sort_by_key(|r| r.offset);

        // Merge adjacent free regions
        let mut i = 0;
        while i < self.free_regions.len() - 1 {
            if self.free_regions[i].offset + self.free_regions[i].size
                == self.free_regions[i + 1].offset
            {
                self.free_regions[i].size += self.free_regions[i + 1].size;
                self.free_regions.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }

    fn reclaim(&mut self, offset: u64, size: u64) {
        if size == 0 {
            return;
        }

        let mut left_index = None; // Some if there is a free region to the left of the deallocated region
        let mut right_index = None; // Some if there is a free region to the right of the deallocated region

        for (i, free_region) in self.free_regions.iter().enumerate() {
            if free_region.offset + free_region.size == offset {
                left_index = Some(i);
            } else if offset + size == free_region.offset {
                right_index = Some(i);
            }
        }

        match (left_index, right_index) {
            // Case A: Merges with both left and right adjacent free regions
            (Some(left), Some(right)) => {
                self.free_regions[left].size += size + self.free_regions[right].size;
                self.free_regions.remove(right);
            }
            // Case B: Merges with only the left adjacent free region
            (Some(left), None) => {
                self.free_regions[left].size += size;
            }
            // Case C: Merges with only the right adjacent free region
            (None, Some(right)) => {
                self.free_regions[right].offset = offset;
                self.free_regions[right].size += size;
            }
            // Case D: No adjacent free regions, so insert and keep sorted by offset
            (None, None) => {
                let region = FreeMegabufferRegion { offset, size };
                self.free_regions.push(region);
                self.free_regions.sort_by_key(|r| r.offset);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static MEGABUFFER_ID: MegabufferId = MegabufferId(0);

    #[test]
    fn test_new_freelist() {
        let freelist = MegabufferFreeList::new(MEGABUFFER_ID, 1024);
        assert_eq!(freelist.free_regions.len(), 1);
        assert_eq!(freelist.free_regions[0].offset, 0);
        assert_eq!(freelist.free_regions[0].size, 1024);
    }

    #[test]
    fn test_carve_free_region() {
        let mut freelist = MegabufferFreeList::new(MEGABUFFER_ID, 1024);
        let carved = freelist.carve_free_region(256);
        assert!(carved.is_some());
        let carved = carved.unwrap();
        assert_eq!(carved.offset, 0);
        assert_eq!(carved.size, 256);
    }

    #[test]
    fn test_carve_out_of_memory() {
        let mut freelist = MegabufferFreeList::new(MEGABUFFER_ID, 512);
        let carved = freelist.carve_free_region(1024);
        assert!(carved.is_none());
    }

    #[test]
    fn test_reclaim_isolated() {
        let mut freelist = MegabufferFreeList {
            megabuffer_id: MEGABUFFER_ID,
            free_regions: vec![
                FreeMegabufferRegion {
                    offset: 0,
                    size: 100,
                },
                FreeMegabufferRegion {
                    offset: 500,
                    size: 100,
                },
            ],
        };

        // Reclaim in the middle without touching neighbors
        freelist.reclaim(250, 50);

        assert_eq!(freelist.free_regions.len(), 3);
        assert_eq!(freelist.free_regions[0].offset, 0);
        assert_eq!(freelist.free_regions[1].offset, 250);
        assert_eq!(freelist.free_regions[1].size, 50);
        assert_eq!(freelist.free_regions[2].offset, 500);
    }

    #[test]
    fn test_reclaim_merge_left() {
        let mut freelist = MegabufferFreeList {
            megabuffer_id: MEGABUFFER_ID,
            free_regions: vec![
                FreeMegabufferRegion {
                    offset: 0,
                    size: 100,
                },
                FreeMegabufferRegion {
                    offset: 500,
                    size: 100,
                },
            ],
        };

        // Reclaim immediately adjacent to the right of the first block [0..100]
        freelist.reclaim(100, 50);

        assert_eq!(freelist.free_regions.len(), 2);
        assert_eq!(freelist.free_regions[0].offset, 0);
        assert_eq!(freelist.free_regions[0].size, 150);
        assert_eq!(freelist.free_regions[1].offset, 500);
    }

    #[test]
    fn test_reclaim_merge_right() {
        let mut freelist = MegabufferFreeList {
            megabuffer_id: MEGABUFFER_ID,
            free_regions: vec![
                FreeMegabufferRegion {
                    offset: 0,
                    size: 100,
                },
                FreeMegabufferRegion {
                    offset: 500,
                    size: 100,
                },
            ],
        };

        // Reclaim immediately adjacent to the left of the second block [500..600]
        freelist.reclaim(450, 50);

        assert_eq!(freelist.free_regions.len(), 2);
        assert_eq!(freelist.free_regions[0].offset, 0);
        assert_eq!(freelist.free_regions[0].size, 100);
        assert_eq!(freelist.free_regions[1].offset, 450);
        assert_eq!(freelist.free_regions[1].size, 150);
    }

    #[test]
    fn test_reclaim_merge_both() {
        let mut freelist = MegabufferFreeList {
            megabuffer_id: MEGABUFFER_ID,
            free_regions: vec![
                FreeMegabufferRegion {
                    offset: 0,
                    size: 100,
                },
                FreeMegabufferRegion {
                    offset: 200,
                    size: 100,
                },
            ],
        };

        // Reclaim bridging the gap [100..200]
        freelist.reclaim(100, 100);

        assert_eq!(freelist.free_regions.len(), 1);
        assert_eq!(freelist.free_regions[0].offset, 0);
        assert_eq!(freelist.free_regions[0].size, 300);
    }

    #[test]
    fn test_defragment_free_regions() {
        let mut freelist = MegabufferFreeList {
            megabuffer_id: MEGABUFFER_ID,
            free_regions: vec![
                FreeMegabufferRegion {
                    offset: 200,
                    size: 100,
                },
                FreeMegabufferRegion {
                    offset: 0,
                    size: 100,
                },
                FreeMegabufferRegion {
                    offset: 100,
                    size: 100,
                },
            ],
        };

        freelist.defragment_free_regions();

        assert_eq!(freelist.free_regions.len(), 1);
        assert_eq!(freelist.free_regions[0].offset, 0);
        assert_eq!(freelist.free_regions[0].size, 300);
    }
}
