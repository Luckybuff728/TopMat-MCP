### B-C-SI 三元计算 payload 参考
```json
{
    "title": "ternary_calculation_{{$date.isoTimestamp}}",
    "description": "{\"task_type\": \"ternary_calculation\",\"tdb_file\": \"/app/exe/topthermo-next/database/B-C-SI-ZR-HF-LA-Y-TI-O.TDB\",\"task_name\": \"ternary_calculation_1\",\"task_path\": \"\",\"condition\": {\"components\": [\"HF\",\"C\",\"SI\"],\"activated_phases\": [\"LIQUID\",\"HCP_A3_HF\",\"DIAMOND_A4\"],\"temperature\": 1100,\"compositions_y\": {\"HF\": 1,\"C\": 0,\"SI\": 0},\"compositions_x\": {\"HF\": 0,\"C\": 1,\"SI\": 0},\"compositions_o\": {\"HF\": 0,\"C\": 0,\"SI\": 1}}}",
    "task_type": "topthermo_next",
    "db_key": "default"
}
```

### Al 点计算 `point_calculation` payload 参考
```json
    {
        "title": "point_calculation_{{$date.isoTimestamp}}",
        "description": "{\"task_type\": \"point_calculation\",\"tdb_file\": \"/app/exe/topthermo-next/database/Al-Si-Mg-Fe-Mn_by_wf.TDB\",\"task_name\": \"al_point_calculation_1\",\"task_path\": \"\",\"condition\": {\"components\": [\"AL\",\"SI\",\"MG\",\"FE\",\"MN\"],\"activated_phases\": [\"LIQUID\",\"FCC_A1\",\"DIAMOND_A4\",\"HCP_A3\",\"BCC_A2\",\"CBCC_A12\",\"BETA_ALMG\",\"EPSILON_ALMG\",\"GAMMA_ALMG\",\"MG2SI\",\"AL5FE2\",\"AL13FE4\",\"ALPHA_ALFESI\",\"BETA_ALFESI\",\"ALPHA_ALFEMNSI\",\"AL4_FEMN\"],\"temperature\": 850,\"compositions\": {\"AL\": 0.93,\"SI\": 0.04,\"MG\": 0.01,\"FE\": 0.015,\"MN\": 0.005}}}",
        "task_type": "topthermo_next",
        "db_key": "default"
    }
```

### Al 二元相图 `binary_equilibrium` payload 参考
```json
{
    "title": "binary_equilibrium_{{$date.isoTimestamp}}",
    "description": "{\"task_type\": \"binary_equilibrium\",\"tdb_file\": \"/app/exe/topthermo-next/database/Al-Si-Mg-Fe-Mn_by_wf.TDB\",\"task_name\": \"al_binary_equilibrium_1\",\"task_path\": \"\",\"activated_elements\": [\"AL\",\"SI\"],\"created\": \"2026-03-07 00:00:00\",\"condition\": {\"activated_phases\": [\"LIQUID\",\"FCC_A1\",\"DIAMOND_A4\"],\"components\": [\"AL\",\"SI\"],\"compositions_start\": {\"AL\": 1.0,\"SI\": 0.0},\"compositions_end\": {\"AL\": 0.7,\"SI\": 0.3},\"temperature_start\": 500.0,\"temperature_end\": 1200.0}}",
    "task_type": "topthermo_next",
    "db_key": "default"
}
```

### Al 线计算 `line_calculation` payload 参考
```json
{
    "title": "line_calculation_{{$date.isoTimestamp}}",
    "description": "{\"task_type\": \"line_calculation\",\"tdb_file\": \"/app/exe/topthermo-next/database/Al-Si-Mg-Fe-Mn_by_wf.TDB\",\"task_name\": \"al_line_calculation_1\",\"task_path\": \"\",\"condition\": {\"components\": [\"AL\",\"SI\",\"MG\",\"FE\",\"MN\"],\"compositions_start\": {\"AL\": 0.93,\"SI\": 0.04,\"MG\": 0.01,\"FE\": 0.015,\"MN\": 0.005},\"compositions_end\": {\"AL\": 0.93,\"SI\": 0.04,\"MG\": 0.01,\"FE\": 0.015,\"MN\": 0.005},\"temperature_start\": 500,\"temperature_end\": 950,\"increments\": 25,\"activated_phases\": [\"LIQUID\",\"FCC_A1\",\"DIAMOND_A4\",\"HCP_A3\",\"BCC_A2\",\"CBCC_A12\",\"BETA_ALMG\",\"EPSILON_ALMG\",\"GAMMA_ALMG\",\"MG2SI\",\"AL5FE2\",\"AL13FE4\",\"ALPHA_ALFESI\",\"BETA_ALFESI\",\"ALPHA_ALFEMNSI\",\"AL4_FEMN\"]}}",
    "task_type": "topthermo_next",
    "db_key": "default"
}
```

### Al 三元计算 `ternary_calculation` payload 参考
```json
{
    "title": "ternary_calculation_{{$date.isoTimestamp}}",
    "description": "{\"task_type\": \"ternary_calculation\",\"tdb_file\": \"/app/exe/topthermo-next/database/Al-Si-Mg-Fe-Mn_by_wf.TDB\",\"task_name\": \"al_ternary_calculation_1\",\"task_path\": \"\",\"condition\": {\"components\": [\"AL\",\"MG\",\"SI\"],\"activated_phases\": [\"LIQUID\",\"FCC_A1\",\"DIAMOND_A4\",\"HCP_A3\",\"BETA_ALMG\",\"EPSILON_ALMG\",\"GAMMA_ALMG\",\"MG2SI\"],\"temperature\": 773,\"compositions_y\": {\"AL\": 1.0,\"MG\": 0.0,\"SI\": 0.0},\"compositions_x\": {\"AL\": 0.0,\"MG\": 1.0,\"SI\": 0.0},\"compositions_o\": {\"AL\": 0.0,\"MG\": 0.0,\"SI\": 1.0}}}",
    "task_type": "topthermo_next",
    "db_key": "default"
}
```

### Al 热力学性质 `thermodynamic_properties` payload 参考
```json
{
    "title": "thermodynamic_properties_{{$date.isoTimestamp}}",
    "description": "{\"task_type\": \"thermodynamic_properties\",\"tdb_file\": \"/app/exe/topthermo-next/database/Al-Si-Mg-Fe-Mn_by_wf.TDB\",\"task_name\": \"al_thermodynamic_properties_1\",\"task_path\": \"\",\"condition\": {\"components\": [\"AL\",\"SI\",\"MG\",\"FE\",\"MN\"],\"activated_phases\": [\"LIQUID\",\"FCC_A1\",\"DIAMOND_A4\",\"HCP_A3\",\"BCC_A2\",\"CBCC_A12\",\"BETA_ALMG\",\"EPSILON_ALMG\",\"GAMMA_ALMG\",\"MG2SI\",\"AL5FE2\",\"AL13FE4\",\"ALPHA_ALFESI\",\"BETA_ALFESI\",\"ALPHA_ALFEMNSI\",\"AL4_FEMN\"],\"compositions_start\": {\"AL\": 0.93,\"SI\": 0.04,\"MG\": 0.01,\"FE\": 0.015,\"MN\": 0.005},\"compositions_end\": {\"AL\": 0.93,\"SI\": 0.04,\"MG\": 0.01,\"FE\": 0.015,\"MN\": 0.005},\"temperature_start\": 500,\"temperature_end\": 950,\"increments\": 25,\"pressure_start\": 5,\"pressure_end\": 5,\"pressure_increments\": 2,\"properties\": [\"GM\",\"HM\",\"SM\",\"CPM\"]}}",
    "task_type": "topthermo_next",
    "db_key": "default"
}
```

### Al Scheil 凝固 `scheil_solidification` payload 参考
```json
{
    "title": "scheil_solidification_{{$date.isoTimestamp}}",
    "description": "{\"task_type\": \"scheil_solidification\",\"tdb_file\": \"/app/exe/topthermo-next/database/Al-Si-Mg-Fe-Mn_by_wf.TDB\",\"task_name\": \"al_scheil_solidification_1\",\"task_path\": \"\",\"condition\": {\"components\": [\"AL\",\"SI\",\"MG\",\"FE\",\"MN\"],\"compositions\": {\"AL\": 0.93,\"SI\": 0.04,\"MG\": 0.01,\"FE\": 0.015,\"MN\": 0.005},\"start_temperature\": 1100.0,\"temperature_step\": 5.0,\"activated_phases\": [\"LIQUID\",\"FCC_A1\",\"DIAMOND_A4\",\"HCP_A3\",\"BCC_A2\",\"CBCC_A12\",\"BETA_ALMG\",\"EPSILON_ALMG\",\"GAMMA_ALMG\",\"MG2SI\",\"AL5FE2\",\"AL13FE4\",\"ALPHA_ALFESI\",\"BETA_ALFESI\",\"ALPHA_ALFEMNSI\",\"AL4_FEMN\"],\"inhibit_phases\": []}}",
    "task_type": "topthermo_next",
    "db_key": "default"
}
```

### Al 沸点计算 `boiling_point` payload 参考
```json
{
    "title": "boiling_point_{{$date.isoTimestamp}}",
    "description": "{\"task_type\": \"boiling_point\",\"tdb_file\": \"/app/exe/topthermo-next/database/Al-Si-Mg-Fe-Mn_by_wf.TDB\",\"task_name\": \"al_boiling_point_1\",\"task_path\": \"\",\"condition\": {\"components\": [\"AL\"],\"pressure\": 101325,\"compositions\": {\"AL\": 1.0},\"temperature_range\": [800,4000]}}",
    "task_type": "topthermo_next",
    "db_key": "default"
}
```