
# Data Sources
Proposing we routinely split remora data into 3 types of samples:

## 1. Cross-Connection Data ("Cross")
This is either _all of_ the data for a field or a sampling from _all of_ the data for a field, regardless of packet directionality or connection association. Specifically we __must__ have multiple connections to use/sample from for some of the later differences between this and single-connection analyses to come out. If we don't have multiple connections then whatever pipeline is doing an analysis should throw some warnings and/or do things differently and not bother creating this type of sample.

## 2. and 3. Single-Connection Forward and Backward Data ("Fwd" and "Bwd")
This is data sampled from single connections, and filtered to either be only client-to-server ("Fwd") or server-to-client ("Bwd").

# Labels
Proposed semantic-labels and (an attempt at) a rigorous definition for each to use when manually labeling protocol fields. This proceeds in a flow-chart-esque manner. __Some fields will have different labels for "Cross" vs. "Fwd" and "Bwd" analysis contexts.__

## 1. Constants
If the field always has a single value in the sample. (Note caveats about weird things like negotiated values where placeholders are used initially and replaced by a constant later)

For Fwd and Bwd data this label _will cover_ data like connection IDs that are constant _within a connection_. And so such fields should be labeled as constant _when analyzed in Fwd and Bwd data_. 

Labeling something as Constant will (generally) skip the Correlator and construct a Generator function that just returns the constant value.

## 2. Length
The field numerically describes the length of another field in the same packet. 

Labeling something as Length will trigger the Stage-2 Correlator to run looking for numerical correlations between this field and the length of each other field (and perhaps sums of lengths of other fields) in the packet. This will in-turn feed into the Generator to build a function with the relevant inputs to match the length field and the length of the field it describes.

## 3. Sequence
The field is numerically correlated with itself across packets. Specifically, it is monotonically increasing across time. This analysis _may_ fail to work on Cross samples. This label includes Timestamps, unless that causes problems.

Labeling a field as a sequence triggers the Stage-2 Correlator to look for and characterize numrical correlations across packets. The Correlator analysis should probably only be conducted on Fwd and Bwd samples, not Cross samples, in order to get as-precise-as-possible a numerical analysis.

## 4. Encrypted
The field has (relatively?) high entropy and is at least 1 byte (not sure about that threshold).

For Cross data this label _will cover_ data like connection IDs that vary across connections. 

Labeling a field as encrypted should make it a prime target for numerical correlation of its length with candidate length field values. Otherwise, the Correlator has nothing to do.

The Generator will (at least initially) assume the field is pseudorandom and so provide pseudorandom data and flag it for potential substitution by encrypted data.

## 5. Flag
The field is nonconstant but small (<= 1 byte, not sure about that threshold). Its characteristics should be consistent for Fwd, Bwd, and Cross samples.

Labeling a field as flag triggers the Stage-2 Correlator to look for existence relationships between the value of this field and the existence or nonexistence of other fields in the same packet (_sort of_ a special case of correlating lengths, not sure if it makes sense to share any actual logic).

Flag fields, if correlated with the existence of fields, will trigger Generator _logic_ that causes fields to exist based on the flag value, or changes the flag value based on selecting fields to exist (TBD Generator logic).

## Connection ID fields ("ID")
ID fields are a special case. They are labeled by finding fields which are classified as "Constant" in Fwd and Bwd samples, but classified as "Encrypted" in Cross samples. This implies they remain constant for a connection but vary across connections.

Labeling a field ID triggers special Generator logic to randomly choose _and then retain_ a value for the ID field for the duration of the connection.

## Flag vs. Encrypted
The boundary between Flag and Encrypted is fuzzy. Size is an awkward and arbitrary delineator. A candidate definition for flag _might_ be "a field that changes the other fields in the packet." In that case, you _could_ label Flag and Encrypted as Encrypted at the Qualifier stage and then separate them based on whether or not we can find a correlation - if we can then it's a flag, if we can't then we leave it as encrypted.

However, now I am worried about fields that we can't just put pseudorandom data in, but also don't match the definition above. I think "not pseudorandom but not matching any other above definition" is going to be a catch-all we need deeper analysis to do things with. __Bottom line: some combination of not-constant but not-high-entropy and smallish size can be a hall-mark of flag, further analysis at later stages can break the label down further.__

