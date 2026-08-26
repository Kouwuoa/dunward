use super::MegabufferId;
use super::region::{AllocatedMegabufferRegion, FreeMegabufferRegion};
use color_eyre::eyre::{Result, eyre};

/// Mutex-guarded CPU state of the [`Megabuffer`]
pub(crate) struct MegabufferFreeList {
    megabuffer_id: MegabufferId,
    free_regions: Vec<FreeMegabufferRegion>,
    total_capacity: u64,
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
            total_capacity,
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

        // Clean up the free region that was split if its size is 0
        if self.free_regions[region_index].size == 0 {
            self.free_regions.remove(region_index);
        }

        Some(new_region)
    }

    pub fn reclaim_region(&mut self, region: AllocatedMegabufferRegion) -> Result<()> {
        if !region.belongs_to_megabuffer_id(self.megabuffer_id) {
            return Err(eyre!(
                "Attempted to reclaim a region that does not belong to this megabuffer"
            ));
        }

        self.reclaim(region.offset(), region.size())?;

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

    pub fn available_bytes(&self) -> u64 {
        self.free_regions.iter().map(|r| r.size).sum()
    }

    pub fn allocated_bytes(&self) -> u64 {
        self.total_capacity - self.available_bytes()
    }

    pub fn total_capacity(&self) -> u64 {
        self.total_capacity
    }

    fn reclaim(&mut self, offset: u64, size: u64) -> Result<()> {
        if size == 0 {
            return Err(eyre!("Size must be greater than zero"));
        }

        if size > self.allocated_bytes() {
            return Err(eyre!("Size must be less than or equal to allocated bytes"));
        }

        if offset + size > self.total_capacity {
            return Err(eyre!(
                "Region extends beyond total capacity of the megabuffer"
            ));
        }

        for free_region in &self.free_regions {
            let free_end = free_region.offset + free_region.size;
            let reclaim_end = offset + size;
            if offset < free_end && reclaim_end > free_region.offset {
                return Err(eyre!(
                    "Attempted to reclaim a region that overlaps with an already free region"
                ));
            }
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

        Ok(())
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
        assert_eq!(freelist.available_bytes(), 1024);
        assert_eq!(freelist.allocated_bytes(), 0);
        assert_eq!(freelist.total_capacity(), 1024);
    }

    #[test]
    fn test_carve_free_region() {
        let mut freelist = MegabufferFreeList::new(MEGABUFFER_ID, 1024);
        let carved = freelist.carve_free_region(256);
        assert!(carved.is_some());
        let carved = carved.unwrap();
        assert_eq!(carved.offset, 0);
        assert_eq!(carved.size, 256);
        assert_eq!(freelist.available_bytes(), 768);
        assert_eq!(freelist.allocated_bytes(), 256);
        assert_eq!(freelist.total_capacity(), 1024);
        assert_eq!(freelist.free_regions.len(), 1);
        assert_eq!(freelist.free_regions[0].offset, 256);
        assert_eq!(freelist.free_regions[0].size, 768);
    }

    #[test]
    fn test_carve_multiple_consecutive() {
        let mut freelist = MegabufferFreeList::new(MEGABUFFER_ID, 1024);

        let r1 = freelist.carve_free_region(256).expect("First carve should succeed");
        assert_eq!(r1.offset, 0);
        assert_eq!(r1.size, 256);
        assert_eq!(freelist.available_bytes(), 768);
        assert_eq!(freelist.allocated_bytes(), 256);

        let r2 = freelist.carve_free_region(256).expect("Second carve should succeed");
        assert_eq!(r2.offset, 256);
        assert_eq!(r2.size, 256);
        assert_eq!(freelist.available_bytes(), 512);
        assert_eq!(freelist.allocated_bytes(), 512);

        let r3 = freelist.carve_free_region(512).expect("Third carve should succeed");
        assert_eq!(r3.offset, 512);
        assert_eq!(r3.size, 512);
        assert_eq!(freelist.available_bytes(), 0);
        assert_eq!(freelist.allocated_bytes(), 1024);
        assert!(freelist.free_regions.is_empty());

        // Further allocations must fail when exhausted
        assert!(freelist.carve_free_region(1).is_none());
    }

    #[test]
    fn test_carve_exact_capacity() {
        let mut freelist = MegabufferFreeList::new(MEGABUFFER_ID, 1024);
        let carved = freelist.carve_free_region(1024);
        assert!(carved.is_some());
        let carved = carved.unwrap();
        assert_eq!(carved.offset, 0);
        assert_eq!(carved.size, 1024);
        assert_eq!(freelist.available_bytes(), 0);
        assert_eq!(freelist.allocated_bytes(), 1024);
        assert!(freelist.free_regions.is_empty());
    }

    #[test]
    fn test_carve_out_of_memory() {
        let mut freelist = MegabufferFreeList::new(MEGABUFFER_ID, 512);
        let carved = freelist.carve_free_region(1024);
        assert!(carved.is_none());
        assert_eq!(freelist.available_bytes(), 512);
        assert_eq!(freelist.allocated_bytes(), 0);
        assert_eq!(freelist.total_capacity(), 512);
    }

    #[test]
    fn test_carve_skips_small_regions_to_find_fit() {
        let mut freelist = MegabufferFreeList {
            megabuffer_id: MEGABUFFER_ID,
            free_regions: vec![
                FreeMegabufferRegion {
                    offset: 0,
                    size: 100,
                },
                FreeMegabufferRegion {
                    offset: 200,
                    size: 300,
                },
            ],
            total_capacity: 500,
        };

        // Request 200: first region (size 100) is too small, second region (size 300) fits
        let carved = freelist.carve_free_region(200).expect("Should find fitting region");
        assert_eq!(carved.offset, 200);
        assert_eq!(carved.size, 200);
        assert_eq!(freelist.free_regions.len(), 2);
        assert_eq!(freelist.free_regions[0].offset, 0);
        assert_eq!(freelist.free_regions[0].size, 100);
        assert_eq!(freelist.free_regions[1].offset, 400);
        assert_eq!(freelist.free_regions[1].size, 100);
        assert_eq!(freelist.available_bytes(), 200);
        assert_eq!(freelist.allocated_bytes(), 300);
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
            total_capacity: 600,
        };

        // Reclaim in the middle without touching neighbors
        freelist.reclaim(250, 50).unwrap();

        assert_eq!(freelist.free_regions.len(), 3);
        assert_eq!(freelist.free_regions[0].offset, 0);
        assert_eq!(freelist.free_regions[0].size, 100);
        assert_eq!(freelist.free_regions[1].offset, 250);
        assert_eq!(freelist.free_regions[1].size, 50);
        assert_eq!(freelist.free_regions[2].offset, 500);
        assert_eq!(freelist.free_regions[2].size, 100);
        assert_eq!(freelist.available_bytes(), 250);
        assert_eq!(freelist.allocated_bytes(), 350);
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
            total_capacity: 1024,
        };

        // Reclaim immediately adjacent to the right of the first block [0..100]
        freelist.reclaim(100, 50).unwrap();

        assert_eq!(freelist.free_regions.len(), 2);
        assert_eq!(freelist.free_regions[0].offset, 0);
        assert_eq!(freelist.free_regions[0].size, 150);
        assert_eq!(freelist.free_regions[1].offset, 500);
        assert_eq!(freelist.free_regions[1].size, 100);
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
            total_capacity: 1024,
        };

        // Reclaim immediately adjacent to the left of the second block [500..600]
        freelist.reclaim(450, 50).unwrap();

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
            total_capacity: 1024,
        };

        // Reclaim bridging the gap [100..200]
        freelist.reclaim(100, 100).unwrap();

        assert_eq!(freelist.free_regions.len(), 1);
        assert_eq!(freelist.free_regions[0].offset, 0);
        assert_eq!(freelist.free_regions[0].size, 300);
    }

    #[test]
    fn test_reclaim_invalid_inputs() {
        let mut freelist = MegabufferFreeList::new(MEGABUFFER_ID, 1024);
        freelist.carve_free_region(256).unwrap();

        // Reclaim size 0 -> error
        assert!(freelist.reclaim(0, 0).is_err());

        // Reclaim size larger than allocated bytes -> error
        assert!(freelist.reclaim(0, 500).is_err());

        // Reclaim beyond total capacity -> error
        assert!(freelist.reclaim(1000, 100).is_err());

        // Reclaim overlapping with already free region [256..1024] -> error
        assert!(freelist.reclaim(200, 100).is_err());
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
            total_capacity: 300,
        };

        freelist.defragment_free_regions();

        assert_eq!(freelist.free_regions.len(), 1);
        assert_eq!(freelist.free_regions[0].offset, 0);
        assert_eq!(freelist.free_regions[0].size, 300);
    }

    #[test]
    fn test_defragment_multiple_disjoint_clusters() {
        let mut freelist = MegabufferFreeList {
            megabuffer_id: MEGABUFFER_ID,
            free_regions: vec![
                FreeMegabufferRegion {
                    offset: 500,
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
                FreeMegabufferRegion {
                    offset: 400,
                    size: 100,
                },
            ],
            total_capacity: 1000,
        };

        freelist.defragment_free_regions();

        assert_eq!(freelist.free_regions.len(), 2);
        assert_eq!(freelist.free_regions[0].offset, 0);
        assert_eq!(freelist.free_regions[0].size, 200);
        assert_eq!(freelist.free_regions[1].offset, 400);
        assert_eq!(freelist.free_regions[1].size, 200);
    }

    #[test]
    fn test_full_alloc_dealloc_lifecycle() {
        let mut freelist = MegabufferFreeList::new(MEGABUFFER_ID, 1000);

        let r1 = freelist.carve_free_region(200).unwrap();
        let r2 = freelist.carve_free_region(300).unwrap();
        let r3 = freelist.carve_free_region(500).unwrap();

        assert_eq!(freelist.available_bytes(), 0);
        assert_eq!(freelist.allocated_bytes(), 1000);
        assert!(freelist.free_regions.is_empty());

        // Reclaim middle region r2 (200..500)
        freelist.reclaim(r2.offset, r2.size).unwrap();
        assert_eq!(freelist.available_bytes(), 300);
        assert_eq!(freelist.allocated_bytes(), 700);
        assert_eq!(freelist.free_regions.len(), 1);
        assert_eq!(freelist.free_regions[0].offset, 200);
        assert_eq!(freelist.free_regions[0].size, 300);

        // Carve smaller chunk from r2's gap
        let r4 = freelist.carve_free_region(100).unwrap();
        assert_eq!(r4.offset, 200);
        assert_eq!(r4.size, 100);
        assert_eq!(freelist.available_bytes(), 200);
        assert_eq!(freelist.free_regions[0].offset, 300);
        assert_eq!(freelist.free_regions[0].size, 200);

        // Reclaim r1 (0..200) -> now free regions are [0..200] and [300..500]
        freelist.reclaim(r1.offset, r1.size).unwrap();
        assert_eq!(freelist.free_regions.len(), 2);
        assert_eq!(freelist.free_regions[0].offset, 0);
        assert_eq!(freelist.free_regions[0].size, 200);
        assert_eq!(freelist.free_regions[1].offset, 300);
        assert_eq!(freelist.free_regions[1].size, 200);

        // Reclaim r4 (200..300) -> bridges left (0..200) and right (300..500)
        freelist.reclaim(r4.offset, r4.size).unwrap();
        assert_eq!(freelist.free_regions.len(), 1);
        assert_eq!(freelist.free_regions[0].offset, 0);
        assert_eq!(freelist.free_regions[0].size, 500);

        // Reclaim r3 (500..1000) -> merges with [0..500] to restore full capacity [0..1000]
        freelist.reclaim(r3.offset, r3.size).unwrap();
        assert_eq!(freelist.free_regions.len(), 1);
        assert_eq!(freelist.free_regions[0].offset, 0);
        assert_eq!(freelist.free_regions[0].size, 1000);
        assert_eq!(freelist.available_bytes(), 1000);
        assert_eq!(freelist.allocated_bytes(), 0);
    }
}
