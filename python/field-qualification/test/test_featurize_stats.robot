*** Settings ***
Library           OperatingSystem
Library           Collections
Library           String
Library           PandasLibrary
Library           Process
Library           CSVLibrary

*** Variables ***
${SCRIPT}         featurization/stats.py
${INPUT}          test/stats-input-remora.json
${OUTPUT_DIR}     test/stats_test_output/
${OUTPUT_CSV}     ${OUTPUT_DIR}stats-input-remora-meta-features.csv
${ROWS}           5

*** Test Cases ***
Verify Featurization
    [Documentation]    This test runs the featurization and verifies the output.
    Run Process        python    ${SCRIPT}    ${INPUT}    --output    ${OUTPUT_DIR}    --force
    File Should Exist  ${OUTPUT_CSV}
    Verify CSV Content

*** Keywords ***
Verify CSV Content
    [Documentation]    Verify the content of the generated CSV file.
    @{feature_data}=  read csv file to list  ${OUTPUT_CSV}
    Log  ${feature_data[0]}
    # Do random spot checks on the file
    Should Be Equal    ${feature_data[0][0]}    name
    Should Be Equal    ${feature_data[0][1]}    label

    ${result}=  Run Process  md5sum  --check  test/test_featurize_stats.checksum    shell=True
    Should Be Equal As Integers  ${result.rc}  0  Checksum of ${OUTPUT_CSV} has changed ${result.stderr}
