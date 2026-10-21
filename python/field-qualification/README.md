# unsupervised-field-qualification

## Running

### Full Pipeline

This is the high level interface to take in one or many Remora JSON files and output a function specification JSON file. 

```bash
python full-pipeline/pipeline.py remora-1.json remora-2.json ...

python full-pipeline/choose_files_from_dir.py ../../data/remora-datasets/ | xargs python full-pipeline/pipeline.py
```


### Featurization

Run this to featurize the data for use my the ML models. Pass a Remora JSON output file to the `stats.py` script.

```bash
python featurization/stats.py /path/to/remora.json
python featurization/stats.py ../../data/remora-datasets/capture_2024-03-26-12-01-24_eth/capture_2024-03-26-12-01-24_eth-quic.json
python featurization/stats.py ../../data/remora-datasets/capture_2024-03-26-12-01-24_eth/capture_2024-03-26-12-01-24_eth-quic.json -n
```

This will produce a CSV file in the `output/` directory with the same base name as the input file. The output is a featurization of the input where the rows are fields on a per-connection basis and the columns are the features. Any feature prefixed with `global_` is a feature computed across all connections of the input file.

## Testing

```bash
make test
```

You can also run the tests directly.

```bash
# Python unit tests.
python tests.py

# Robot tests.
python -m robot --outputdir test/robot/ test/test_featurize_stats.robot
```

## Machine Learning

Takes in the output CSV from `featurization/stats.py`.

```bash
python qualifier-pipeline/QualifierPipeline.py --output temp --data-root=./output --training-patterns "*rtp*.csv,*quic*.csv" --testing-patterns "*rtp*.csv,*quic*.csv"
```

This will output a CSV file where the rows are field names, the columns are features, and the values are confidence scores in the range `[0, 1.0]`.

## Goals from discussion with Paul

Define statistical functions that will produce meta-features for feeding an unsupervised (e.g. clustering) model. Features includes, but are not limited to, min/max, entropy, uniqueness (need to nail down what that means exactly), standard deviation, variance, variance of length (probably other stats based on length also apply, e.g. min/max).

## Additional Feature Ideas

* growth/increase/decrease
* correlation
  * actively being worked on in `field-correlation` repo.
* per connection stats
* updates to uniqueness
  * normalizing based on data set size and field value range (e.g. 0-255 for 8-bit)
  * associating with connections/directions
* average entropy for larger data fields

---
Copyright 2024 Two Six Technologies.




```
python qualifier-pipeline/QualifierPipeline.py --output temp --data-root=../../data/remora-datasets/sampled-min10-csv -r "rtp*.csv,quic*.csv" -e "dnp3*.csv,quic*.csv"


python qualifier-pipeline/QualifierPipeline.py --output temp --data-root=./output -r "*rtp*.csv,*quic*.csv" -e "*rtp*.csv,*quic*.csv"


python qualifier-pipeline/QualifierPipeline.py --output temp --data-root=./output -r "rtp*.csv,quic*.csv" -e "*rtp*.csv,*quic*.csv"
```
