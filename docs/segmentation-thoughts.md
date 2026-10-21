# Segmentation Thoughts

## Matrix / Image Representations

Initially structure the data for all packets of a protocol as a 2D matrix of bits - rows are individual packets, columns are bit indices, truncate to normalize the number of columns. For later analyses, represent `0-bits` as `-1.0` rather than `0.0`, but `1-bits` as `1.0`.

### Vertical Aggregation
Averaging the bits of a column effectively represents entropy of the bit index across all the packets; with the `-1` representation above high-entropy indices will tend towards `0` and low-entropy indices will tend towards `-1` or `1`. For fixed-position protocols (i.e. fields do not shift which index they start/end at) a vertical aggregation should make it easy to segment high-entropy fields based on a heuristic threshold. Taking the absolute value of the averaged data will provide `1` for low-entropy indices and `0` for high-entropy indices.

### Finding App Data Segments
Assume app data segments are 1) always high entropy (either due to encryption or compression) and 2) always `n` bytes (e.g. 4-bytes because we will assume a not-super-optimized data serialization scheme). Then a heuristic algorithm for identifying app data segments could scan the absolute-value vertically-averaged (see above) data and look for "seeds" that are 4-byte segments with the minimum summed value (high-entropy bit indices tend towards 0, so we want the 32 lowest-value / highest-entropy contiguous bits). From a seed, walk forward and backward along bit indices until the index value rises above the threshold for a "random" bit. Alternatively, have a multi-bit lookahead to let this approach "push through" an app data bit that was coincidentally not that random. An app data segment should probably also always be a multiple of 8 in length (that is, full bytes and not ending with a partial byte).

Potentially existing CV / image processing models would do the above as some sort of object detection?

### Finding Constant Segments
Finding constant segments is similar to finding app data segments, except now the index values to look for are those _above_ some threshold which indicates very low (or zero) entropy. In this case, however, we have no minimum size since constant segments could be as small as individual bit-flags. If we simply apply an inverted version of the app data segment finding algorithm we can pull all the contiguous constant-value "chunks." 

These chunks will likely contain errors of two types: 

#### 1) False aggregations of multiple constant-value segments 
If multiple constant-value fields always appear in a contiguous span there is really nothing more a segmentation algorithm can do. If these really are all constant (i.e. we trust our data corpus) then they are a _de facto_ constant semantic entity and there is no theoretical basis for separating them into more spans. __Helpfully, we have no reason to need to resolve this false aggregation.__

#### 2) False inclusion of non-constant segment portions
If a non-constant segment adjoins a constant segment _and has a constant prefix or suffix_ then we may mistakenly include the constant portion into a constant segment. E.g. imagine a 1-byte length field denoting the length of a data segment, but in all packets the length happens to fall between `0-63`. The first two bits of the length field will always be `0` and could be accidentally included in a constant segment directly before the length field in the packet. 

Similar to #1, there does not seem to be a theoretical basis for resolving these false inclusions aside from some external concept of how a protocol _should_ look. E.g. if a field is not variable-length encoded and appears to be 6 bits long, it probably _should_ be a full byte. __This seems like a bad path to travel down and should probably be seen as a weakness in the data corpus not the segmentation algorithms.__


### Finding Other Segments
If we have found all the app data and constant segments we should just be left with bits that vary _some_ but are either not high enough entropy or not large enough contiguous spans to satisfy the app data definition. Where these adjoin constant segments the border should be clear and easy. Where these adjoin app data segments the boundary must be based on a different in entropy (otherwise the app data segment would have encompassed the bit?) and could be fuzzier: maybe this was a mistake and the app data segment just _happened_ to have a lower-entropy at its beginning/end. 

Internal boundaries for these segments will be more complicated: e.g. given two adjacent varying fields, how can we correctly segment them? My first thought is to recurse on the "pre-segment" and try other types of analyses rather than just the vertical averaging, but with the analysis narrowed to the contiguous column indices of the pre-segment. Some random ideas for analyses:

1. Sequential analysis as object detection: a monotonically increasing field (e.g. a sequence number of timestamp) represented in binary should appear roughly as a contiguous blob (if it's actually increasing by 1 each time it should have no "holes").
1. Distribution analysis: if we can generate hypotheses for how the segment might be broken up, we could try statistical tests against the values to see if they follow some distribution. However, without a prior belief of what that distribution _should_ look like this seems like a non-starter.
1. Motif / semi-constant analysis: find repeated motifs / continuous segments of repeated bits across subsets of packets. These could be repeated values of a segment and if they vary at different times from a different segment, the mismatch in when one segment changes vs. another could provide evidence for segmentation. E.g. there are two fields, A which can be a value of A1 or A2 and B which can be a value of B1 or B2. If A1 stays constant for a set of packets while B alternates between B1 and B2, and subsequently A changes to A2 but B continues alternating, we could potentially identify that A and B are segments of bits that both vary but vary at different rates / frequencies and so should be different segments.
