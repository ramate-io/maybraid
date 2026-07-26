# Richmond Building Components

This create contains various scene components for Richmond buildings. 

## Swept Components

We have not yet defined a sweeping tool. The plan is to make it take linear segments [-1.0, 1.0] and fill them along a line. 

## Partitions

- **Linear Normalization:** linear components are normalized to the following spaces:
    - Z = [-0.2, 0.2]
    - Y = [0.0, 1.0]
    - X = [-1.0, 1.0]
    - Subsegments normalized to X = [-1.0, 0.8]
- **Angular Normalization:** angular components follow a similar normalization on along the arc but attach to different start and end points at different angles.
    - Thickness is the same swept Z = [-0.2, 0.2]
    - A 180 degree arc sweep will sweep through the -Z from X = -1.0 to X = 1.0
    - A 90 degree arc sweep will sweep through the -Z from X = -1.0 to X = 0.0
    - A 15 degree arc sweep will sweep through the -Z from X = -1.0 to X = cos(15) - 1.0, Z = -sin(15)
- **Header Components:** header components are used for smaller vertical spaces. They are normalized to the following spaces:
    - Z = [-0.2, 0.2]
    - Y = [0.0, 0.3]
    - X = [-1.0, 1.0]