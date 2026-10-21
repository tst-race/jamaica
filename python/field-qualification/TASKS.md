# Tasks

- [ ] Consolidate `percent_increase` and `percent_increase` into single feature where a negative value implies decrease. Compute a count: increase add one, decrease minus one, no change add zero. Divide by num samples minus one or whatever, just normalize between -1 and 1.
- [ ] Currently global features are generated per capture. Instead, should we be generating them on the fly across all input files and captures? The issue with the current method is that some fields show up as contant some of the time (e.g. `rtp.marker`, `rtp.p_type`, `rtp.padding`), which doesn't really make sense. _Should_ just be able to consolidate the CSV values, assuming normalization is done correctly. Maybe setup some tests to validate this.
- [ ] The features `global_percent_increase` and `global_percent_descrease` don't make any sense. We would never really expect a value to trend in _any_ direction across connections. Maybe compute the average here, or something else. Detect a saw tooth type pattern?
- [ ] Compute global stats on the fly across all incoming connections. Otherwise we have multiple global values which aren't truly global.
