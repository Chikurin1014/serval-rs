# Concept

Serval makes no demands on what your device sends.
It takes what it receives as it is, and turns it into data as you tell it to.

```mermaid
flowchart LR
    subgraph maps[Maps]
        map1[Map 1]
        map2[Map 2]
        map3[Map 3]
    end

    subgraph graphs[Graphs]
        graph1[/Graph 1/]
        graph2[/Graph 2/]
    end

    subgraph basicGroup[Basic features]
        port[/Serial port/]
        data[(Data with labels)]
        maps
        graphs
    end

    subgraph taskGroup[Plugin]
        task1[Task 1]
        task2[Task 2]
        task3[Task 3]
    end

    data <-- read / push --> maps
    port -- push (raw_bytes) --> data
    data -- read --> graphs
    task1 <-- read / send --> port
    task2 -- send --> port
    task2 <-- read / push --> data
    task3 <-- read / push --> data
```

* **[Data](data-types.md)**: values under **labels**, each label holding one
  **type**, each value with the time it arrived.
* **[Maps](maps.md)**: turn the values of labels into values of other labels.
* **[Graphs](../usage/graph.md)**: plot labels, each graph showing the types it supports.
* **[Tasks](tasks.md)**: scripts of your own, which read and push data, and talk to the port.
