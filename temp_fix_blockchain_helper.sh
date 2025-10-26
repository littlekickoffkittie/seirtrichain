#!/bin/bash

# Fix the create_dummy_transaction helper
sed -i '/fn create_dummy_transaction/,/^    }$/ {
    /let children = t.subdivide();/a\        let children: [Triangle; 3] = [\
            children_vec[0].clone(),\
            children_vec[1].clone(),\
            children_vec[2].clone(),\
        ];
    s/let children = t.subdivide();/let children_vec = t.subdivide();/
}' ~/siertrichain/src/blockchain.rs
