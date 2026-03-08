# Al-Si-Mg-Fe-Mn 任务参考配置

本目录提供针对 `database/Al-Si-Mg-Fe-Mn_by_wf.TDB` 的参考配置。

设计原则：

- 以 Al-rich 铸造/铝合金场景为主
- 二元任务优先选 `AL-SI`，因为相图最稳定、最直观
- 三元任务优先选 `AL-MG-SI`，因为对应典型析出/共晶子系统
- 点计算、线计算、热力学性质、Scheil 任务使用 5 元 Al-rich 合金成分，显式纳入 Fe/Mn 相关金属间化合物

默认 5 元示例成分：

- `AL = 0.93`
- `SI = 0.04`
- `MG = 0.01`
- `FE = 0.015`
- `MN = 0.005`

该组配置已完成 7 类任务回归：

- `point_calculation` 可运行
- `binary_equilibrium` 可运行
- `line_calculation` 可运行
- `ternary_calculation` 可运行
- `thermodynamic_properties` 可运行
- `scheil_solidification` 可运行
- `boiling_point` 可运行

如果后续需要更偏 6xxx、3xx.x 或高 Fe 再生铝场景，可以在此基础上调整成分与激活相集合。
