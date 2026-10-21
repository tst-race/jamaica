.PHONY: all
all:

.PHONY: venv
venv:
	if [ ! -d "./venv" ]; then python3 -m venv venv; fi
	@echo "run \`source venv/bin/activate\` to enter virtualenv"

.PHONY: install_reqs
install_reqs:
	python -m pip install -r requirements.txt

.PHONY: update_reqs
update_reqs:
	python -m pip freeze > requirements.txt

.PHONY: format
format:
	find . -name '*.py' -not -path './venv/*' | xargs python -m black

.PHONY: lint
lint:
	find . -name '*.py' -not -path './venv/*' | xargs python -m pylint

.PHONY: type_check
type_check:
	find . -name '*.py' -not -path './venv/*' | xargs python -m mypy

.PHONY: run_stats
run_stats:
	grep -v '^#' inputs.txt | xargs -n1 python featurization/stats.py -f

.PHONY: run_stats_new
run_stats_new:
	grep -v '^#' inputs.txt | xargs -n1 python featurization/stats.py --headless

.PHONY: test
test:
	python tests.py
	python -m robot --outputdir test/robot/ test/test_featurize_stats.robot
