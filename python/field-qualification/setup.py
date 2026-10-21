from setuptools import setup, find_packages

setup(
    name="correlator",
    version="0.1",
    packages=find_packages(),
    include_package_data=True,
    entry_points="""
        [console_scripts]
        correlator=correlator.cli.cli:main
        refinery=refinery.cli.cli:main
    """,
)


# setup(
#     name='refinery',
#     version='0.1',
#     packages=find_packages(),
#     include_package_data=True,
#     entry_points='''
#         [console_scripts]
#         refinery=refinery.cli.cli:main
#     ''',
# )
